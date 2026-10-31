//! MCP connector OAuth 2.0 public-client + PKCE flow (docs/07 §6, RFC 8252).
//!
//! The desktop client is a **public client** (ADR-060: no static secrets, no
//! `client_secret`): it generates a PKCE verifier/challenge (S256), binds an
//! ephemeral `127.0.0.1` loopback redirect, opens the **system browser** at the
//! connector's `authorize_url`, receives the authorization code on the loopback
//! listener, then exchanges it at `token_url` (and refreshes it later). Tokens are
//! handed to the host for the existing keychain IPC (`oauth.save`); this module
//! never logs them and never writes them to disk itself.
//!
//! Security notes:
//! - endpoints are validated by [`zk_contract_connector::ConnectorOAuth::validate`]
//!   (https everywhere, plain `http://` only for loopback, no userinfo/fragment);
//! - the loopback listener binds `127.0.0.1:0` only (never `0.0.0.0`);
//! - `state` is checked before accepting the code, and the token endpoint is
//!   called with redirect following disabled (no SSRF via redirects);
//! - HTTP client errors surface as status codes only (never the response body),
//!   so a provider error page cannot leak token material into logs/UI.

#![forbid(unsafe_code)]

use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zk_contract_connector::ConnectorOAuth;

/// Loopback callback path used by the redirect URI.
pub const CALLBACK_PATH: &str = "/callback";
/// Default whole-flow timeout (system browser interaction included).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 180;
/// Smallest accepted whole-flow timeout.
pub const MIN_TIMEOUT_SECONDS: u64 = 30;
/// Largest accepted whole-flow timeout.
pub const MAX_TIMEOUT_SECONDS: u64 = 600;
/// Per-HTTP-request timeout for token calls.
const TOKEN_HTTP_TIMEOUT_SECONDS: u64 = 30;
/// Max bytes read from one loopback HTTP request.
const MAX_HTTP_REQUEST_BYTES: usize = 8 * 1024;

/// Stable error codes surfaced through IPC (no token material).
pub const ERR_LISTEN_FAILED: &str = "OAUTH_LISTEN_FAILED";
/// The browser did not complete the flow before the deadline.
pub const ERR_TIMEOUT: &str = "OAUTH_TIMEOUT";
/// The callback `state` did not match the generated one.
pub const ERR_STATE_MISMATCH: &str = "OAUTH_STATE_MISMATCH";
/// The callback carried no code (or an unusable request).
pub const ERR_CALLBACK_INVALID: &str = "OAUTH_CALLBACK_INVALID";
/// The token response could not be parsed.
pub const ERR_TOKEN_BAD_RESPONSE: &str = "OAUTH_TOKEN_BAD_RESPONSE";
/// The token request failed at the transport layer.
pub const ERR_TOKEN_NETWORK: &str = "OAUTH_TOKEN_NETWORK";
/// A refresh was requested for a token without a refresh token.
pub const ERR_REFRESH_NO_TOKEN: &str = "OAUTH_REFRESH_NO_TOKEN";

/// One PKCE verifier/challenge pair (RFC 7636, S256 only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkcePair {
    /// High-entropy verifier (unreserved characters only).
    pub verifier: String,
    /// `BASE64URL(SHA256(verifier))`, no padding.
    pub challenge: String,
}

impl PkcePair {
    /// Generate a fresh pair from OS randomness (`uuid` v4).
    #[must_use]
    pub fn generate() -> Self {
        // Two random 128-bit values => 64 hex chars (within RFC 7636's 43..=128).
        let verifier = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let challenge = code_challenge(&verifier);
        Self {
            verifier,
            challenge,
        }
    }
}

/// `BASE64URL(SHA256(verifier))` without padding (PKCE S256).
#[must_use]
pub fn code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// OAuth token set as stored in the client keychain (JSON object).
///
/// `refresh_token`/`scope` are optional: some providers only issue access tokens.
/// The previous TS flow typed both, so a provider without a refresh token is
/// reported as "needs re-authorization" by callers rather than silently kept.
#[derive(Clone, PartialEq, Eq)]
pub struct OAuthTokens {
    /// Bearer access token.
    pub access_token: String,
    /// Optional refresh token.
    pub refresh_token: Option<String>,
    /// Token type (defaults to `Bearer`).
    pub token_type: String,
    /// Absolute expiry in Unix milliseconds.
    pub expires_at: i64,
    /// Optional granted scope string.
    pub scope: Option<String>,
}

impl std::fmt::Debug for OAuthTokens {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OAuthTokens")
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .field("scope", &self.scope)
            .finish()
    }
}

impl OAuthTokens {
    /// Parse a token-endpoint response (`access_token`, `expires_in`, ...).
    ///
    /// # Errors
    ///
    /// [`ERR_TOKEN_BAD_RESPONSE`] when required fields are missing/blank.
    pub fn from_response(value: &Value, now_ms: i64) -> Result<Self, String> {
        let access_token = value
            .get("access_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| ERR_TOKEN_BAD_RESPONSE.to_owned())?;
        let refresh_token = value
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_owned);
        let token_type = value
            .get("token_type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
            .unwrap_or("Bearer")
            .to_owned();
        let expires_in = value
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(3_600)
            .max(1);
        let expires_at = now_ms.saturating_add(expiry_millis(expires_in));
        let scope = value
            .get("scope")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|scope| !scope.is_empty())
            .map(str::to_owned);
        Ok(Self {
            access_token: access_token.to_owned(),
            refresh_token,
            token_type,
            expires_at,
            scope,
        })
    }

    /// Parse the keychain JSON string produced by [`Self::to_json`] (or the old TS shape).
    ///
    /// # Errors
    ///
    /// [`ERR_TOKEN_BAD_RESPONSE`] when the stored payload is not an object with
    /// `access_token`/`expires_at`.
    pub fn from_stored_json(raw: &str) -> Result<Self, String> {
        let value: Value =
            serde_json::from_str(raw).map_err(|_| ERR_TOKEN_BAD_RESPONSE.to_owned())?;
        let access_token = value
            .get("access_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| ERR_TOKEN_BAD_RESPONSE.to_owned())?;
        let expires_at = value
            .get("expires_at")
            .and_then(Value::as_i64)
            .ok_or_else(|| ERR_TOKEN_BAD_RESPONSE.to_owned())?;
        let refresh_token = value
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_owned);
        let token_type = value
            .get("token_type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
            .unwrap_or("Bearer")
            .to_owned();
        let scope = value
            .get("scope")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok(Self {
            access_token: access_token.to_owned(),
            refresh_token,
            token_type,
            expires_at,
            scope,
        })
    }

    /// Serialize for the keychain (`oauth.save` payload).
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut value = json!({
            "access_token": self.access_token,
            "token_type": self.token_type,
            "expires_at": self.expires_at,
        });
        if let (Some(refresh), Some(object)) = (&self.refresh_token, value.as_object_mut()) {
            object.insert("refresh_token".to_owned(), Value::String(refresh.clone()));
        }
        if let (Some(scope), Some(object)) = (&self.scope, value.as_object_mut()) {
            object.insert("scope".to_owned(), Value::String(scope.clone()));
        }
        value
    }
}

fn expiry_millis(expires_in_seconds: u64) -> i64 {
    i64::try_from(expires_in_seconds)
        .unwrap_or(i64::MAX / 1_000)
        .saturating_mul(1_000)
}

/// Current Unix time in milliseconds (never panics: pre-epoch clocks yield 0).
#[must_use]
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Percent-encode one RFC 3986 unreserved-set-preserving component.
#[must_use]
pub fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Percent-decode a query/form component (best-effort, never panics).
#[must_use]
pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0_usize;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'+' => {
                decoded.push(b' ');
                index = index.saturating_add(1);
            }
            b'%' => {
                let high = bytes.get(index.saturating_add(1)).copied();
                let low = bytes.get(index.saturating_add(2)).copied();
                match (high, low) {
                    (Some(high), Some(low)) => {
                        if let (Some(high), Some(low)) = (hex_value(high), hex_value(low)) {
                            decoded.push(high.saturating_mul(16).saturating_add(low));
                            index = index.saturating_add(3);
                        } else {
                            decoded.push(byte);
                            index = index.saturating_add(1);
                        }
                    }
                    _ => {
                        decoded.push(byte);
                        index = index.saturating_add(1);
                    }
                }
            }
            _ => {
                decoded.push(byte);
                index = index.saturating_add(1);
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte.saturating_sub(b'0')),
        b'a'..=b'f' => Some(byte.saturating_sub(b'a').saturating_add(10)),
        b'A'..=b'F' => Some(byte.saturating_sub(b'A').saturating_add(10)),
        _ => None,
    }
}

fn form_encode(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Parse a `key=value&...` query/form string.
fn parse_pairs(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (percent_decode(key), percent_decode(value)),
            None => (percent_decode(pair), String::new()),
        })
        .collect()
}

fn pair_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// An already-bound loopback redirect receiver.
#[derive(Debug)]
pub struct LoopbackReceiver {
    listener: TcpListener,
    redirect_uri: String,
}

impl LoopbackReceiver {
    /// Bind `127.0.0.1:0` and derive the RFC 8252 redirect URI.
    ///
    /// # Errors
    ///
    /// [`ERR_LISTEN_FAILED`] when the loopback socket cannot be bound.
    pub fn bind() -> Result<Self, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("{ERR_LISTEN_FAILED}: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("{ERR_LISTEN_FAILED}: {error}"))?;
        let redirect_uri = format!("http://127.0.0.1:{}{CALLBACK_PATH}", address.port());
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("{ERR_LISTEN_FAILED}: {error}"))?;
        Ok(Self {
            listener,
            redirect_uri,
        })
    }

    /// The `http://127.0.0.1:<ephemeral>/callback` URI handed to the provider.
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Wait for the redirect callback and return the authorization code.
    ///
    /// # Errors
    ///
    /// [`ERR_TIMEOUT`], [`ERR_STATE_MISMATCH`], [`ERR_CALLBACK_INVALID`] or the
    /// provider error code (`OAUTH_DENIED:*`) from the callback.
    pub fn wait_for_code(&self, expected_state: &str, deadline: Instant) -> Result<String, String> {
        loop {
            if Instant::now() >= deadline {
                return Err(ERR_TIMEOUT.to_owned());
            }
            match self.listener.accept() {
                Ok((stream, _address)) => match handle_callback(stream, expected_state) {
                    CallbackOutcome::Ignored => {}
                    CallbackOutcome::Code(code) => return Ok(code),
                    CallbackOutcome::Failed(error) => return Err(error),
                },
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(format!("{ERR_LISTEN_FAILED}: {error}")),
            }
        }
    }
}

enum CallbackOutcome {
    Ignored,
    Code(String),
    Failed(String),
}

fn handle_callback(mut stream: std::net::TcpStream, expected_state: &str) -> CallbackOutcome {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let request = match read_http_request(&mut stream) {
        Ok(request) => request,
        Err(_) => {
            write_http_response(&mut stream, "400 Bad Request", "Bad request.");
            return CallbackOutcome::Failed(ERR_CALLBACK_INVALID.to_owned());
        }
    };
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    if method != "GET" {
        write_http_response(&mut stream, "405 Method Not Allowed", "Method not allowed.");
        return CallbackOutcome::Ignored;
    }
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    if path != CALLBACK_PATH {
        write_http_response(&mut stream, "404 Not Found", "Not found.");
        return CallbackOutcome::Ignored;
    }
    let pairs = parse_pairs(query);
    if let Some(error) = pair_value(&pairs, "error") {
        let description = pair_value(&pairs, "error_description").unwrap_or_default();
        write_http_response(
            &mut stream,
            "400 Bad Request",
            "Authorization was denied. You can close this window.",
        );
        return CallbackOutcome::Failed(format!(
            "OAUTH_DENIED:{}:{}",
            sanitize_token(error),
            sanitize_token(description)
        ));
    }
    let state = pair_value(&pairs, "state").unwrap_or_default();
    if state != expected_state {
        write_http_response(
            &mut stream,
            "400 Bad Request",
            "State mismatch. You can close this window.",
        );
        return CallbackOutcome::Failed(ERR_STATE_MISMATCH.to_owned());
    }
    let code = pair_value(&pairs, "code")
        .unwrap_or_default()
        .trim()
        .to_owned();
    if code.is_empty() {
        write_http_response(
            &mut stream,
            "400 Bad Request",
            "Missing authorization code.",
        );
        return CallbackOutcome::Failed(ERR_CALLBACK_INVALID.to_owned());
    }
    write_http_response(
        &mut stream,
        "200 OK",
        "Authorization received. You can close this window and return to ZeoKeel.",
    );
    CallbackOutcome::Code(code)
}

fn sanitize_token(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .take(64)
        .collect()
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Result<String, String> {
    let mut buffer = vec![0_u8; MAX_HTTP_REQUEST_BYTES];
    let mut filled = 0_usize;
    loop {
        let start = filled.min(MAX_HTTP_REQUEST_BYTES.saturating_sub(1));
        let Some(target) = buffer.get_mut(start..) else {
            break;
        };
        let read = stream.read(target).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        filled = filled.saturating_add(read);
        let received = buffer
            .get(..filled)
            .map(String::from_utf8_lossy)
            .unwrap_or_default();
        if received.contains("\r\n\r\n") || filled >= MAX_HTTP_REQUEST_BYTES {
            return Ok(received.into_owned());
        }
    }
    let received = buffer
        .get(..filled)
        .map(String::from_utf8_lossy)
        .unwrap_or_default();
    Ok(received.into_owned())
}

fn write_http_response(stream: &mut std::net::TcpStream, status: &str, body: &str) {
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>ZeoKeel</title></head><body><p>{body}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn build_authorize_url(
    config: &ConnectorOAuth,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
) -> String {
    let mut url = config.authorize_url.trim().to_owned();
    url.push(if url.contains('?') { '&' } else { '?' });
    let scope = config.scopes.join(" ");
    let params = vec![
        ("response_type".to_owned(), "code".to_owned()),
        ("client_id".to_owned(), config.client_id.trim().to_owned()),
        ("redirect_uri".to_owned(), redirect_uri.to_owned()),
        ("scope".to_owned(), scope),
        ("state".to_owned(), state.to_owned()),
        ("code_challenge".to_owned(), challenge.to_owned()),
        ("code_challenge_method".to_owned(), "S256".to_owned()),
    ];
    url.push_str(&form_encode(&params));
    url
}

fn token_request(token_url: &str, pairs: &[(String, String)]) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(TOKEN_HTTP_TIMEOUT_SECONDS))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("{ERR_TOKEN_NETWORK}: {error}"))?;
    let response = client
        .post(token_url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .body(form_encode(pairs))
        .send()
        .map_err(|error| format!("{ERR_TOKEN_NETWORK}: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        // Never include the body: provider error pages may echo credentials.
        return Err(format!("OAUTH_TOKEN_HTTP_{}", status.as_u16()));
    }
    response
        .json::<Value>()
        .map_err(|_| ERR_TOKEN_BAD_RESPONSE.to_owned())
}

fn exchange_code(
    config: &ConnectorOAuth,
    redirect_uri: &str,
    code: &str,
    verifier: &str,
    now_ms: i64,
) -> Result<OAuthTokens, String> {
    let pairs = vec![
        ("grant_type".to_owned(), "authorization_code".to_owned()),
        ("code".to_owned(), code.to_owned()),
        ("client_id".to_owned(), config.client_id.trim().to_owned()),
        ("redirect_uri".to_owned(), redirect_uri.to_owned()),
        ("code_verifier".to_owned(), verifier.to_owned()),
    ];
    let body = token_request(config.token_url.trim(), &pairs)?;
    OAuthTokens::from_response(&body, now_ms)
}

/// Run the full authorization-code + PKCE flow.
///
/// `open` receives the authorize URL and must start the system browser without
/// blocking (it must not wait for the user); the flow then waits for the loopback
/// callback until `timeout` elapses.
///
/// # Errors
///
/// `OAUTH_VALIDATION:*` for invalid metadata, plus [`ERR_LISTEN_FAILED`],
/// [`ERR_TIMEOUT`], [`ERR_STATE_MISMATCH`], [`ERR_CALLBACK_INVALID`],
/// `OAUTH_DENIED:*`, `OAUTH_TOKEN_HTTP_<status>`, [`ERR_TOKEN_NETWORK`],
/// [`ERR_TOKEN_BAD_RESPONSE`].
pub fn authorize(
    config: &ConnectorOAuth,
    timeout: Duration,
    now_ms: i64,
    open: &dyn Fn(&str) -> Result<(), String>,
) -> Result<OAuthTokens, String> {
    config.validate()?;
    let receiver = LoopbackReceiver::bind()?;
    let pkce = PkcePair::generate();
    let state = uuid::Uuid::new_v4().to_string();
    let authorize_url =
        build_authorize_url(config, receiver.redirect_uri(), &pkce.challenge, &state);
    open(&authorize_url)?;
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    let code = receiver.wait_for_code(&state, deadline)?;
    exchange_code(
        config,
        receiver.redirect_uri(),
        &code,
        &pkce.verifier,
        now_ms,
    )
}

/// Refresh a stored token set (rotation-aware: keeps the old refresh token when
/// the provider does not rotate it).
///
/// # Errors
///
/// `OAUTH_VALIDATION:*` for invalid metadata, [`ERR_REFRESH_NO_TOKEN`] when no
/// refresh token is stored, plus `OAUTH_TOKEN_HTTP_<status>`/[`ERR_TOKEN_NETWORK`]/
/// [`ERR_TOKEN_BAD_RESPONSE`].
pub fn refresh(
    config: &ConnectorOAuth,
    current: &OAuthTokens,
    now_ms: i64,
) -> Result<OAuthTokens, String> {
    config.validate()?;
    let refresh_token = current
        .refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ERR_REFRESH_NO_TOKEN.to_owned())?;
    let pairs = vec![
        ("grant_type".to_owned(), "refresh_token".to_owned()),
        ("refresh_token".to_owned(), refresh_token.to_owned()),
        ("client_id".to_owned(), config.client_id.trim().to_owned()),
    ];
    let body = token_request(config.token_url.trim(), &pairs)?;
    let mut next = OAuthTokens::from_response(&body, now_ms)?;
    if next.refresh_token.is_none() {
        next.refresh_token = current.refresh_token.clone();
    }
    Ok(next)
}

/// Open the authorize URL in the system browser (host-side implementation).
pub fn open_system_browser(url: &str) -> Result<(), String> {
    #[cfg(test)]
    if let Some(opener) = test_opener() {
        return opener(url);
    }
    crate::ipc::open_platform_browser_checked(url)
}

#[cfg(test)]
pub(crate) type TestOpener = std::sync::Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

#[cfg(test)]
fn test_opener_slot() -> &'static std::sync::Mutex<Option<TestOpener>> {
    static SLOT: std::sync::OnceLock<std::sync::Mutex<Option<TestOpener>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(None))
}

#[cfg(test)]
fn test_opener() -> Option<TestOpener> {
    test_opener_slot().lock().ok().and_then(|slot| slot.clone())
}

/// Test-only hook: replace the system-browser opener (loopback flows only).
#[cfg(test)]
pub(crate) fn set_test_opener(opener: Option<TestOpener>) {
    if let Ok(mut slot) = test_opener_slot().lock() {
        *slot = opener;
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Deterministic local OAuth server + scripted "browser" for tests (localhost only).
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::arithmetic_side_effects
    )]

    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use serde_json::json;

    /// Behaviour knobs for the fake authorize/token server.
    #[derive(Debug, Clone)]
    pub struct FakeOAuthConfig {
        /// Redirect with a wrong `state` (negative test).
        pub wrong_state: bool,
        /// Reject the PKCE verifier at the token endpoint (negative test).
        pub reject_pkce: bool,
        /// Return this HTTP status on the token endpoint (negative test).
        pub token_status: u16,
        /// Body returned with `token_status` (must never leak to callers/logs).
        pub token_error_body: String,
        /// Reject a reused (rotated) refresh token.
        pub reject_reused_refresh: bool,
    }

    impl Default for FakeOAuthConfig {
        fn default() -> Self {
            Self {
                wrong_state: false,
                reject_pkce: false,
                token_status: 200,
                token_error_body: "provider error body SECRET-DO-NOT-LOG".to_owned(),
                reject_reused_refresh: true,
            }
        }
    }

    #[derive(Debug, Default)]
    struct FakeState {
        codes: HashMap<String, (String, String, String)>, // code -> (state, challenge, redirect)
        redirect_uri: Option<String>,
        authorize_url: Option<String>,
        refresh_token: Option<String>,
        access_counter: u32,
        saw_pkce_mismatch: bool,
        reused_refresh: bool,
    }

    /// A fake OAuth provider bound to 127.0.0.1 (no external network).
    pub struct FakeOAuthServer {
        authorize_url: String,
        token_url: String,
        state: Arc<Mutex<FakeState>>,
        handle: Option<std::thread::JoinHandle<()>>,
        stop: Arc<std::sync::atomic::AtomicBool>,
    }

    impl FakeOAuthServer {
        pub fn start(config: FakeOAuthConfig) -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fake oauth");
            let port = listener.local_addr().expect("addr").port();
            let state = Arc::new(Mutex::new(FakeState::default()));
            let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let thread_state = Arc::clone(&state);
            let thread_stop = Arc::clone(&stop);
            let config = Arc::new(config);
            let handle = std::thread::spawn(move || {
                while !thread_stop.load(std::sync::atomic::Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            serve(stream, &thread_state, &config);
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                authorize_url: format!("http://127.0.0.1:{port}/authorize"),
                token_url: format!("http://127.0.0.1:{port}/token"),
                state,
                handle: Some(handle),
                stop,
            }
        }

        pub fn authorize_url(&self) -> &str {
            &self.authorize_url
        }

        pub fn token_url(&self) -> &str {
            &self.token_url
        }

        /// The `redirect_uri` the browser used at `/authorize`.
        pub fn observed_redirect_uri(&self) -> Option<String> {
            self.state
                .lock()
                .ok()
                .and_then(|state| state.redirect_uri.clone())
        }

        /// The authorize URL the client asked the browser to open.
        pub fn observed_authorize_url(&self) -> Option<String> {
            self.state
                .lock()
                .ok()
                .and_then(|state| state.authorize_url.clone())
        }

        pub fn saw_pkce_mismatch(&self) -> bool {
            self.state
                .lock()
                .map(|state| state.saw_pkce_mismatch)
                .unwrap_or(false)
        }

        pub fn saw_reused_refresh(&self) -> bool {
            self.state
                .lock()
                .map(|state| state.reused_refresh)
                .unwrap_or(false)
        }
    }

    impl Drop for FakeOAuthServer {
        fn drop(&mut self) {
            self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
            // Wake the accept loop so the thread can exit.
            let address = self
                .authorize_url
                .split("//")
                .nth(1)
                .and_then(|rest| rest.split('/').next())
                .unwrap_or_default()
                .to_owned();
            let _ = std::net::TcpStream::connect(address);
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn serve(mut stream: TcpStream, state: &Arc<Mutex<FakeState>>, config: &FakeOAuthConfig) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let Some(request) = read_request(&mut stream) else {
            return;
        };
        let (method, target, body) = request;
        if method == "GET" && target.starts_with("/authorize") {
            let query = target.split_once('?').map(|(_, query)| query).unwrap_or("");
            let pairs = parse_pairs(query);
            let redirect = pair_value(&pairs, "redirect_uri")
                .unwrap_or_default()
                .to_owned();
            let state_value = pair_value(&pairs, "state").unwrap_or_default().to_owned();
            let challenge = pair_value(&pairs, "code_challenge")
                .unwrap_or_default()
                .to_owned();
            let code = format!("code-{}", uuid::Uuid::new_v4().simple());
            if let Ok(mut state) = state.lock() {
                state.redirect_uri = Some(redirect.clone());
                state.authorize_url = Some(target.to_owned());
                state.codes.insert(
                    code.clone(),
                    (state_value.clone(), challenge, redirect.clone()),
                );
            }
            let redirect_state = if config.wrong_state {
                format!("{state_value}-tampered")
            } else {
                state_value
            };
            let location = format!(
                "{redirect}?code={}&state={}",
                percent_encode(&code),
                percent_encode(&redirect_state)
            );
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(response.as_bytes());
            return;
        }
        if method == "POST" && target.starts_with("/token") {
            let pairs = parse_pairs(&body);
            let grant = pair_value(&pairs, "grant_type").unwrap_or_default();
            let mut status = config.token_status;
            let mut payload = json!({});
            if status == 200 {
                if config.reject_pkce {
                    status = 400;
                } else if grant == "authorization_code" {
                    let code = pair_value(&pairs, "code").unwrap_or_default().to_owned();
                    let verifier = pair_value(&pairs, "code_verifier").unwrap_or_default();
                    let entry = state
                        .lock()
                        .ok()
                        .and_then(|state| state.codes.get(&code).cloned());
                    match entry {
                        Some((_, challenge, _)) => {
                            if super::code_challenge(verifier) != challenge {
                                if let Ok(mut state) = state.lock() {
                                    state.saw_pkce_mismatch = true;
                                }
                                status = 400;
                            } else {
                                let refresh = format!("refresh-{}", uuid::Uuid::new_v4().simple());
                                let mut access = 0_u32;
                                if let Ok(mut state) = state.lock() {
                                    state.refresh_token = Some(refresh.clone());
                                    state.access_counter = state.access_counter.saturating_add(1);
                                    access = state.access_counter;
                                }
                                payload = json!({
                                    "access_token": format!("access-{access}"),
                                    "refresh_token": refresh,
                                    "token_type": "Bearer",
                                    "expires_in": 3600,
                                    "scope": "mcp.read",
                                });
                            }
                        }
                        None => status = 400,
                    }
                } else if grant == "refresh_token" {
                    let presented = pair_value(&pairs, "refresh_token").unwrap_or_default();
                    let current_refresh = state
                        .lock()
                        .ok()
                        .and_then(|state| state.refresh_token.clone());
                    let accepted = match (&current_refresh, config.reject_reused_refresh) {
                        (Some(current), _) if current == presented => true,
                        (Some(_current), false) if presented.starts_with("refresh-") => true,
                        _ => false,
                    };
                    if !accepted {
                        if let Ok(mut state) = state.lock() {
                            state.reused_refresh = true;
                        }
                        status = 400;
                    } else {
                        let refresh = format!("refresh-{}", uuid::Uuid::new_v4().simple());
                        let mut access = 0_u32;
                        if let Ok(mut state) = state.lock() {
                            state.refresh_token = Some(refresh.clone());
                            state.access_counter = state.access_counter.saturating_add(1);
                            access = state.access_counter;
                        }
                        payload = json!({
                            "access_token": format!("access-{access}"),
                            "refresh_token": refresh,
                            "token_type": "Bearer",
                            "expires_in": 60,
                        });
                    }
                } else {
                    status = 400;
                }
            }
            let (status_line, response_body) = if status == 200 {
                (
                    "200 OK".to_owned(),
                    serde_json::to_string(&payload).unwrap_or_default(),
                )
            } else {
                (
                    format!("{status} Bad Request"),
                    config.token_error_body.clone(),
                )
            };
            let response = format!(
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            return;
        }
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    }

    fn read_request(stream: &mut TcpStream) -> Option<(String, String, String)> {
        let mut buffer = vec![0_u8; 16 * 1024];
        let mut filled = 0_usize;
        loop {
            let read = stream.read(&mut buffer[filled..]).ok()?;
            if read == 0 {
                break;
            }
            filled = filled.saturating_add(read);
            let text = String::from_utf8_lossy(&buffer[..filled]).into_owned();
            if let Some(index) = text.find("\r\n\r\n") {
                let head = text.get(..index)?.to_owned();
                let body = text.get(index.saturating_add(4)..)?.to_owned();
                let mut parts = head.split_whitespace();
                let method = parts.next()?.to_owned();
                let target = parts.next()?.to_owned();
                if body.is_empty() {
                    return Some((method, target, String::new()));
                }
                return Some((method, target, body));
            }
        }
        None
    }

    /// Simulate the system browser: GET the authorize URL and follow the loopback
    /// redirect, exactly like a browser would.
    pub fn browser_open(url: &str) {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("browser client");
        let response = client.get(url).send().expect("browser GET authorize");
        assert_eq!(response.status().as_u16(), 200, "loopback callback page");
    }

    /// An opener that starts the simulated browser on a background thread and returns.
    pub fn spawning_opener(delay: Duration) -> impl Fn(&str) -> Result<(), String> + Send + Sync {
        move |url: &str| {
            let url = url.to_owned();
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                browser_open(&url);
            });
            Ok(())
        }
    }

    fn parse_pairs(raw: &str) -> Vec<(String, String)> {
        raw.split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| match pair.split_once('=') {
                Some((key, value)) => (super::percent_decode(key), super::percent_decode(value)),
                None => (super::percent_decode(pair), String::new()),
            })
            .collect()
    }

    fn pair_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
        pairs
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    fn percent_encode(value: &str) -> String {
        super::percent_encode(value)
    }

    /// Convenience: a valid connector OAuth metadata pointing at the fake server.
    pub fn config_for(server: &FakeOAuthServer) -> zk_contract_connector::ConnectorOAuth {
        zk_contract_connector::ConnectorOAuth {
            client_id: "zk-public-client".to_owned(),
            scopes: vec!["mcp.read".to_owned()],
            redirect_uri: String::new(),
            authorize_url: server.authorize_url().to_owned(),
            token_url: server.token_url().to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

    use super::test_support::{config_for, spawning_opener, FakeOAuthConfig, FakeOAuthServer};
    use super::*;

    #[test]
    fn pkce_s256_matches_rfc7636_test_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            code_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let pair = PkcePair::generate();
        assert!((43..=128).contains(&pair.verifier.len()));
        assert!(pair
            .verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')));
        assert_eq!(pair.challenge, code_challenge(&pair.verifier));
        assert!(!pair.challenge.contains('='));
    }

    #[test]
    fn percent_encoding_round_trips_and_decodes_form_values() {
        assert_eq!(percent_encode("a b/c+d~e"), "a%20b%2Fc%2Bd~e");
        assert_eq!(percent_decode("a%20b%2Fc%2Bd"), "a b/c+d");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("%zz"), "%zz");
        for value in ["mcp.read mcp.write", "a&b=c?d#e", "汉字"] {
            assert_eq!(percent_decode(&percent_encode(value)), value);
        }
    }

    #[test]
    fn authorize_url_carries_pkce_state_and_loopback_redirect() {
        let config = ConnectorOAuth {
            client_id: "zk-public-client".to_owned(),
            scopes: vec!["mcp.read".to_owned(), "mcp.write".to_owned()],
            redirect_uri: String::new(),
            authorize_url: "https://auth.example.invalid/authorize?prompt=consent".to_owned(),
            token_url: "https://auth.example.invalid/token".to_owned(),
        };
        let url = build_authorize_url(
            &config,
            "http://127.0.0.1:41234/callback",
            "challenge",
            "state-1",
        );
        assert!(url.starts_with("https://auth.example.invalid/authorize?prompt=consent&"));
        assert!(url.contains("response_type=code"), "{url}");
        assert!(url.contains("client_id=zk-public-client"), "{url}");
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A41234%2Fcallback"));
        assert!(url.contains("scope=mcp.read%20mcp.write"));
        assert!(url.contains("code_challenge=challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state-1"));
        assert!(!url.contains("client_secret"), "public client only: {url}");
    }

    #[test]
    fn full_authorize_flow_uses_loopback_and_pkce_and_returns_tokens() {
        let server = FakeOAuthServer::start(FakeOAuthConfig::default());
        let config = config_for(&server);
        let opener = spawning_opener(Duration::ZERO);
        let now = now_ms();
        let tokens = authorize(&config, Duration::from_secs(10), now, &opener).expect("authorize");
        assert!(tokens.access_token.starts_with("access-"));
        assert!(tokens.refresh_token.is_some());
        assert_eq!(tokens.token_type, "Bearer");
        assert!(tokens.expires_at > now);
        assert_eq!(tokens.scope.as_deref(), Some("mcp.read"));

        // RFC 8252: redirect is 127.0.0.1 with an ephemeral port + /callback.
        let redirect = server.observed_redirect_uri().expect("redirect observed");
        assert!(redirect.starts_with("http://127.0.0.1:"), "{redirect}");
        assert!(redirect.ends_with(CALLBACK_PATH), "{redirect}");
        let observed = server.observed_authorize_url().expect("authorize observed");
        assert!(observed.contains("code_challenge_method=S256"));
        assert!(!observed.contains("grant_type"));
        assert!(!server.saw_pkce_mismatch(), "PKCE verifier must match");

        let stored = tokens.to_json();
        let parsed = OAuthTokens::from_stored_json(&stored.to_string()).expect("stored parse");
        assert_eq!(parsed.access_token, tokens.access_token);
        assert_eq!(parsed.refresh_token, tokens.refresh_token);
    }

    #[test]
    fn configured_redirect_uri_is_validated_and_never_used() {
        let server = FakeOAuthServer::start(FakeOAuthConfig::default());
        let mut config = config_for(&server);
        config.redirect_uri = "https://app.example.invalid/callback".to_owned();
        let opener = spawning_opener(Duration::ZERO);
        let error = authorize(&config, Duration::from_secs(5), now_ms(), &opener)
            .expect_err("non-loopback redirect must be rejected");
        assert!(error.starts_with("OAUTH_VALIDATION"), "{error}");
        // Loopback-configured redirect is accepted but the flow still uses its own port.
        config.redirect_uri = "http://127.0.0.1:1/callback".to_owned();
        let tokens = authorize(&config, Duration::from_secs(10), now_ms(), &opener)
            .expect("loopback redirect config accepted");
        assert!(tokens.access_token.starts_with("access-"));
    }

    #[test]
    fn rejects_insecure_or_malformed_endpoints() {
        let mut config = ConnectorOAuth {
            client_id: "c".to_owned(),
            scopes: vec![],
            redirect_uri: String::new(),
            authorize_url: "http://auth.example.invalid/authorize".to_owned(),
            token_url: "https://auth.example.invalid/token".to_owned(),
        };
        let opener = |_url: &str| Ok(());
        assert!(
            authorize(&config, Duration::from_secs(1), now_ms(), &opener)
                .expect_err("http public authorize endpoint")
                .starts_with("OAUTH_VALIDATION")
        );
        config.authorize_url = "https://auth.example.invalid/authorize".to_owned();
        config.token_url = "http://token.example.invalid/token".to_owned();
        assert!(
            authorize(&config, Duration::from_secs(1), now_ms(), &opener)
                .expect_err("http public token endpoint")
                .starts_with("OAUTH_VALIDATION")
        );
        config.token_url = "https://auth.example.invalid/token".to_owned();
        config.client_id = String::new();
        assert!(authorize(&config, Duration::from_secs(1), now_ms(), &opener).is_err());
    }

    #[test]
    fn state_mismatch_fails_closed_without_tokens() {
        let server = FakeOAuthServer::start(FakeOAuthConfig {
            wrong_state: true,
            ..FakeOAuthConfig::default()
        });
        let config = config_for(&server);
        let opener = spawning_opener(Duration::ZERO);
        let error = authorize(&config, Duration::from_secs(10), now_ms(), &opener)
            .expect_err("tampered state must fail");
        assert_eq!(error, ERR_STATE_MISMATCH);
    }

    #[test]
    fn token_endpoint_rejections_do_not_leak_response_body() {
        let server = FakeOAuthServer::start(FakeOAuthConfig {
            token_status: 500,
            ..FakeOAuthConfig::default()
        });
        let config = config_for(&server);
        let opener = spawning_opener(Duration::ZERO);
        let error = authorize(&config, Duration::from_secs(10), now_ms(), &opener)
            .expect_err("500 must fail");
        assert_eq!(error, "OAUTH_TOKEN_HTTP_500");
        assert!(
            !error.contains("SECRET"),
            "provider body must never reach callers: {error}"
        );
    }

    #[test]
    fn pkce_verifier_mismatch_is_rejected_by_the_provider() {
        let server = FakeOAuthServer::start(FakeOAuthConfig {
            reject_pkce: true,
            ..FakeOAuthConfig::default()
        });
        let config = config_for(&server);
        let opener = spawning_opener(Duration::ZERO);
        let error = authorize(&config, Duration::from_secs(10), now_ms(), &opener)
            .expect_err("PKCE mismatch must fail");
        assert_eq!(error, "OAUTH_TOKEN_HTTP_400");
    }

    #[test]
    fn flow_times_out_when_the_browser_never_returns() {
        let config = ConnectorOAuth {
            client_id: "c".to_owned(),
            scopes: vec![],
            redirect_uri: String::new(),
            authorize_url: "https://auth.example.invalid/authorize".to_owned(),
            token_url: "https://auth.example.invalid/token".to_owned(),
        };
        // Opener that never launches a browser (test-only closure).
        let opener = |_url: &str| Ok(());
        let started = Instant::now();
        let error = authorize(&config, Duration::from_millis(300), now_ms(), &opener)
            .expect_err("must time out");
        assert_eq!(error, ERR_TIMEOUT);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn refresh_rotates_tokens_and_rejects_reuse_of_the_old_refresh_token() {
        let server = FakeOAuthServer::start(FakeOAuthConfig::default());
        let config = config_for(&server);
        let opener = spawning_opener(Duration::ZERO);
        let tokens =
            authorize(&config, Duration::from_secs(10), now_ms(), &opener).expect("authorize");
        let old_refresh = tokens.refresh_token.clone().expect("refresh token");

        let refreshed = refresh(&config, &tokens, now_ms()).expect("refresh");
        assert_ne!(refreshed.access_token, tokens.access_token);
        assert!(refreshed.expires_at > 0, "refresh returns a new expiry");
        assert_ne!(
            refreshed.refresh_token.as_deref(),
            Some(old_refresh.as_str()),
            "provider rotates refresh tokens"
        );
        assert!(refreshed.expires_at > 0);

        // Reusing the rotated (old) refresh token must fail and not produce tokens.
        let stale = OAuthTokens {
            refresh_token: Some(old_refresh),
            ..tokens
        };
        let error = refresh(&config, &stale, now_ms()).expect_err("reuse must fail");
        assert_eq!(error, "OAUTH_TOKEN_HTTP_400");
        assert!(server.saw_reused_refresh());
    }

    #[test]
    fn refresh_without_refresh_token_is_rejected() {
        let config = ConnectorOAuth {
            client_id: "c".to_owned(),
            scopes: vec![],
            redirect_uri: String::new(),
            authorize_url: "https://auth.example.invalid/authorize".to_owned(),
            token_url: "https://auth.example.invalid/token".to_owned(),
        };
        let tokens = OAuthTokens {
            access_token: "a".to_owned(),
            refresh_token: None,
            token_type: "Bearer".to_owned(),
            expires_at: 0,
            scope: None,
        };
        assert_eq!(
            refresh(&config, &tokens, now_ms()).expect_err("no refresh token"),
            ERR_REFRESH_NO_TOKEN
        );
    }

    #[test]
    fn token_debug_is_redacted_and_status_shapes_are_parseable() {
        let tokens = OAuthTokens {
            access_token: "ACCESS-SECRET".to_owned(),
            refresh_token: Some("REFRESH-SECRET".to_owned()),
            token_type: "Bearer".to_owned(),
            expires_at: 123,
            scope: Some("mcp.read".to_owned()),
        };
        let debug = format!("{tokens:?}");
        assert!(!debug.contains("ACCESS-SECRET"), "{debug}");
        assert!(!debug.contains("REFRESH-SECRET"), "{debug}");
        assert!(debug.contains("[redacted]"), "{debug}");
        assert!(!tokens.to_json().to_string().contains("client_secret"));
    }

    #[test]
    fn stored_token_shapes_are_backwards_compatible() {
        // Current shape (with token_type/scope) and old TS shape both parse.
        let old = r#"{"access_token":"a","refresh_token":"r","expires_at":1700000000000}"#;
        let parsed = OAuthTokens::from_stored_json(old).expect("legacy parse");
        assert_eq!(parsed.access_token, "a");
        assert_eq!(parsed.refresh_token.as_deref(), Some("r"));
        assert_eq!(parsed.token_type, "Bearer");
        assert_eq!(parsed.expires_at, 1_700_000_000_000);
        // Missing access token / expiry is rejected.
        for bad in [r#"{"expires_at":1}"#, r#"{"access_token":"a"}"#, "not json"] {
            assert!(OAuthTokens::from_stored_json(bad).is_err(), "{bad}");
        }
        // Response without refresh_token is still valid (Option).
        let body = serde_json::json!({"access_token":"a","expires_in":10});
        let parsed = OAuthTokens::from_response(&body, 0).expect("response parse");
        assert!(parsed.refresh_token.is_none());
        assert_eq!(parsed.expires_at, 10_000);
        let body = serde_json::json!({"access_token":"a","expires_in":3600});
        let fresh = OAuthTokens::from_response(&body, 0).expect("response parse");
        assert_eq!(fresh.expires_at, 3_600_000);
    }
}
