//! Enterprise update manifest / download / portal (`srv.updates`).
//!
//! Contract: `docs/04` §2.7 (manifest with stable ETag + conditional 304,
//! Range/206 downloads), `docs/09` §5 (zero-script portal fed by
//! `CLIENT-RELEASE.json` assets, narrowed by channel `platforms`) and
//! `docs/09` §7.3/§7.4 (forced/min_supported, `426 CLIENT_TOO_OLD`, ed25519
//! manifest signature policy).
//!
//! # Signature policy (honest, never faked)
//!
//! The manifest always carries `signature` and `signature_status`:
//!
//! - `ZEOKEEL_UPDATES_SIGNATURE_KEY` unset + release has no signature →
//!   `"unconfigured"` (`signature: null`): this server does not sign manifests.
//! - key unset + release has a signature → `"unverified"` (echoed, client can
//!   verify with its pinned key; the server cannot).
//! - key set + release has no signature → **500 `UPDATE_SIGNATURE_REQUIRED`**
//!   (fail-closed: a signing deployment must ship signed manifests).
//! - key set + invalid signature → **500 `UPDATE_SIGNATURE_INVALID`**
//!   (fail-closed), otherwise `"verified"`.
//!
//! The signed bytes are [`canonical_manifest_bytes`]: the release JSON object
//! with the top-level `signature` field removed, serialized compactly by
//! `serde_json` (sorted keys). Release tooling must sign exactly that.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zk_kit_server::{
    error, request_id_for_request, upgrade_required_response, AppState, ServerError,
};

/// Environment variable holding the ed25519 public key (base64 or hex).
pub const SIGNATURE_KEY_ENV: &str = "ZEOKEEL_UPDATES_SIGNATURE_KEY";
/// Environment override for the manifest `min_supported` (authoritative when set).
pub const MIN_SUPPORTED_ENV: &str = "ZEOKEEL_UPDATES_MIN_SUPPORTED";
/// Environment override forcing updates (`1`/`true`).
pub const FORCED_ENV: &str = "ZEOKEEL_UPDATES_FORCED";

/// Update + portal routes.
///
/// `/` and `/portal` both serve the zero-script download portal (`docs/09` §5).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(portal))
        .route("/portal", get(portal))
        .route("/api/updates/manifest", get(manifest))
        .route("/api/updates/client/{artifact}", get(download))
}

fn update_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("ZEOKEEL_UPDATE_DIR").unwrap_or_else(|_| "deploy/updates".to_owned()),
    )
}

fn release() -> Result<Value, ServerError> {
    let path = update_dir().join("CLIENT-RELEASE.json");
    let text = std::fs::read_to_string(path)
        .map_err(|_| error(StatusCode::NOT_FOUND, "UPDATE_UNAVAILABLE"))?;
    serde_json::from_str(&text)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))
}

fn channel_config() -> Value {
    std::fs::read_to_string(update_dir().join("CHANNEL.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}))
}

fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn client_header(headers: &HeaderMap) -> Option<(String, String, String)> {
    let raw = headers.get("x-zeokeel-client")?.to_str().ok()?;
    let mut parts = raw.split(';');
    let version = parts.next()?.trim().to_owned();
    let channel = parts.next()?.trim().to_owned();
    let platform = parts.next()?.trim().to_owned();
    if version.is_empty() || channel.is_empty() || platform.is_empty() {
        None
    } else {
        Some((version, channel, platform))
    }
}

/// Effective `min_supported`: explicit setting wins over release metadata.
fn min_supported(release: &Value) -> String {
    setting(MIN_SUPPORTED_ENV).unwrap_or_else(|| {
        release
            .get("min_supported")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    })
}

/// Effective `forced`: setting / release metadata, or "client below min".
fn forced_for(release: &Value, client_below_min: bool) -> bool {
    let configured = setting(FORCED_ENV)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"));
    configured
        .or_else(|| release.get("forced").and_then(Value::as_bool))
        .unwrap_or(false)
        || client_below_min
}

/// Compare dotted numeric versions (`0.10.0` > `0.9.9`). Non-numeric parts make
/// the comparison false (unknown ordering is not "too old").
fn version_lt(client: &str, floor: &str) -> bool {
    let parse = |value: &str| -> Option<Vec<u64>> {
        value
            .trim()
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()
    };
    let (Some(mut left), Some(mut right)) = (parse(client), parse(floor)) else {
        return false;
    };
    let width = left.len().max(right.len());
    left.resize(width, 0);
    right.resize(width, 0);
    left < right
}

/// Canonical signed bytes: release object without the top-level `signature`.
///
/// # Errors
///
/// [`ServerError`] when the release is not a JSON object or cannot serialize.
pub fn canonical_manifest_bytes(release: &Value) -> Result<Vec<u8>, ServerError> {
    let mut object = release
        .as_object()
        .cloned()
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))?;
    object.remove("signature");
    let canonical = Value::Object(object);
    serde_json::to_vec(&canonical)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))
}

fn decode_key(raw: &str) -> Option<VerifyingKey> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .or_else(|_| hex_decode(raw))
        .ok()?;
    let array: [u8; 32] = bytes.as_slice().try_into().ok()?;
    VerifyingKey::from_bytes(&array).ok()
}

fn hex_decode(raw: &str) -> Result<Vec<u8>, ()> {
    let raw = raw.trim();
    if !raw.len().is_multiple_of(2) {
        return Err(());
    }
    raw.as_bytes()
        .chunks(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).map_err(|_| ())?;
            u8::from_str_radix(text, 16).map_err(|_| ())
        })
        .collect()
}

fn decode_signature(raw: &str) -> Option<Signature> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .or_else(|_| hex_decode(raw))
        .ok()?;
    Signature::from_slice(&bytes).ok()
}

/// Evaluate the manifest signature policy; returns `(signature_status, signature)`.
fn signature_policy(release: &Value) -> Result<(&'static str, Value), ServerError> {
    let signature = release.get("signature").cloned().unwrap_or(Value::Null);
    let configured = setting(SIGNATURE_KEY_ENV);
    match (configured, signature.as_str()) {
        (None, None) => Ok(("unconfigured", Value::Null)),
        (None, Some(_)) => Ok(("unverified", signature)),
        (Some(_), None) => Err(error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "UPDATE_SIGNATURE_REQUIRED",
        )),
        (Some(key), Some(raw)) => {
            let invalid = || {
                error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "UPDATE_SIGNATURE_INVALID",
                )
            };
            let verifying = decode_key(&key).ok_or_else(invalid)?;
            let parsed = decode_signature(raw).ok_or_else(invalid)?;
            let canonical = canonical_manifest_bytes(release)?;
            verifying
                .verify_strict(&canonical, &parsed)
                .map_err(|_| invalid())?;
            Ok(("verified", signature))
        }
    }
}

fn etag_for(body: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(body);
    format!("\"{:x}\"", hash.finalize())
}

fn if_none_match_matches(header: Option<&str>, etag: &str) -> bool {
    let Some(header) = header else {
        return false;
    };
    header
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == etag || candidate == "*")
}

/// `GET /api/updates/manifest` — stable ETag/304, forced/min_supported,
/// signature policy, `426 CLIENT_TOO_OLD` for unsupported clients.
async fn manifest(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ServerError> {
    let Some((client_version, channel, platform)) = client_header(&headers) else {
        return Err(error(StatusCode::BAD_REQUEST, "CLIENT_HEADER_REQUIRED"));
    };
    let mut release = release()?;
    let release_channel = release
        .get("channel_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let release_edition = release
        .get("edition")
        .and_then(Value::as_str)
        .unwrap_or("enterprise");
    if channel != release_channel {
        return Err(error(StatusCode::CONFLICT, "CHANNEL_MISMATCH"));
    }
    if release_edition != "enterprise" {
        return Err(error(StatusCode::CONFLICT, "EDITION_MISMATCH"));
    }
    let min_supported = min_supported(&release);
    let current_client = release
        .get("client_version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let below_min = !min_supported.is_empty() && version_lt(&client_version, &min_supported);
    if client_version != current_client {
        // Below the supported floor is a forced upgrade (426); any other 1:1
        // mismatch stays a channel/config error (409).
        if below_min {
            let request_id = request_id_for_request(&headers);
            return Ok(upgrade_required_response(
                &request_id,
                Some(&client_version),
                &min_supported,
                "client version is below the minimum supported version",
            ));
        }
        return Err(error(StatusCode::CONFLICT, "CLIENT_VERSION_MISMATCH"));
    }
    let (signature_status, signature) = signature_policy(&release)?;
    let forced = forced_for(&release, below_min);
    let previous = release.get("previous").cloned().unwrap_or(Value::Null);
    let object = release
        .as_object_mut()
        .ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))?;
    object.insert(
        "client".into(),
        json!({ "version": client_version, "channel": channel, "platform": platform }),
    );
    object.insert("forced".into(), json!(forced));
    object.insert("min_supported".into(), json!(min_supported));
    object.insert("previous".into(), previous);
    object.insert("signature".into(), signature);
    object.insert("signature_status".into(), json!(signature_status));
    let body = serde_json::to_vec(&release)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))?;
    let etag = etag_for(&body);
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok());
    if if_none_match_matches(if_none_match, &etag) {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        if let Ok(value) = HeaderValue::from_str(&etag) {
            response.headers_mut().insert(header::ETAG, value);
        }
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        return Ok(response);
    }
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(value) = HeaderValue::from_str(signature_status) {
        response.headers_mut().insert("x-zeokeel-signature", value);
    }
    Ok(response)
}

/// Parsed single byte range.
enum ByteRange {
    /// Serve the whole entity (no/invalid/multi range header).
    Full,
    /// Inclusive `start..=end` slice.
    Partial { start: u64, end: u64 },
    /// Start beyond the entity; respond 416 with `Content-Range: bytes */len`.
    Unsatisfiable,
}

fn parse_range(raw: &str, total: u64) -> ByteRange {
    let Some(list) = raw.trim().strip_prefix("bytes=") else {
        return ByteRange::Full;
    };
    // Multi-range responses are optional in HTTP; serve the full entity instead.
    if list.contains(',') {
        return ByteRange::Full;
    }
    let Some((start_text, end_text)) = list.split_once('-') else {
        return ByteRange::Full;
    };
    let start_text = start_text.trim();
    let end_text = end_text.trim();
    if total == 0 {
        return ByteRange::Unsatisfiable;
    }
    let last = total.saturating_sub(1);
    if start_text.is_empty() {
        // Suffix range `bytes=-N`: last N bytes.
        let Ok(suffix) = end_text.parse::<u64>() else {
            return ByteRange::Full;
        };
        if suffix == 0 {
            return ByteRange::Unsatisfiable;
        }
        let length = suffix.min(total);
        return ByteRange::Partial {
            start: total.saturating_sub(length),
            end: last,
        };
    }
    let Ok(start) = start_text.parse::<u64>() else {
        return ByteRange::Full;
    };
    if start > last {
        return ByteRange::Unsatisfiable;
    }
    let end = if end_text.is_empty() {
        last
    } else {
        let Ok(end) = end_text.parse::<u64>() else {
            return ByteRange::Full;
        };
        end.min(last)
    };
    if end < start {
        return ByteRange::Unsatisfiable;
    }
    ByteRange::Partial { start, end }
}

fn download_headers(response: &mut Response, length: u64) {
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if let Ok(value) = HeaderValue::from_str(&length.to_string()) {
        response.headers_mut().insert(header::CONTENT_LENGTH, value);
    }
}

fn range_unsatisfiable(total: u64) -> Response {
    let mut response = Response::new(Body::from(
        json!({ "ok": false, "error": "RANGE_NOT_SATISFIABLE" }).to_string(),
    ));
    *response.status_mut() = StatusCode::RANGE_NOT_SATISFIABLE;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(value) = HeaderValue::from_str(&format!("bytes */{total}")) {
        response.headers_mut().insert(header::CONTENT_RANGE, value);
    }
    response
}

/// `GET /api/updates/client/{artifact}` — octet-stream with Range/206 support.
async fn download(
    State(_state): State<AppState>,
    AxumPath(artifact): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, ServerError> {
    if artifact.is_empty()
        || artifact.contains("..")
        || artifact.contains('/')
        || artifact.contains('\\')
    {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let path = update_dir().join("clients").join(&artifact);
    let bytes = std::fs::read(&path).map_err(|_| error(StatusCode::NOT_FOUND, "NOT_FOUND"))?;
    let total = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let requested = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(|raw| parse_range(raw, total));
    match requested {
        None | Some(ByteRange::Full) => {
            let mut response = Response::new(Body::from(bytes));
            *response.status_mut() = StatusCode::OK;
            download_headers(&mut response, total);
            Ok(response)
        }
        Some(ByteRange::Unsatisfiable) => Ok(range_unsatisfiable(total)),
        Some(ByteRange::Partial { start, end }) => {
            let (Ok(from), Ok(to)) = (usize::try_from(start), usize::try_from(end)) else {
                return Ok(range_unsatisfiable(total));
            };
            let Some(slice) = bytes.get(from..=to) else {
                return Ok(range_unsatisfiable(total));
            };
            let length = u64::try_from(slice.len()).unwrap_or(0);
            let mut response = Response::new(Body::from(slice.to_vec()));
            *response.status_mut() = StatusCode::PARTIAL_CONTENT;
            download_headers(&mut response, length);
            if let Ok(value) = HeaderValue::from_str(&format!("bytes {start}-{end}/{total}")) {
                response.headers_mut().insert(header::CONTENT_RANGE, value);
            }
            Ok(response)
        }
    }
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

struct Platform {
    key: &'static str,
    label: &'static str,
}

fn platform_for_asset(name: &str) -> Option<Platform> {
    let lower = name.to_ascii_lowercase();
    if lower.contains("linux") || lower.ends_with(".appimage") {
        return Some(Platform {
            key: "linux",
            label: "Linux",
        });
    }
    if lower.contains("windows") || lower.ends_with(".exe") || lower.ends_with(".msi") {
        return Some(Platform {
            key: "windows",
            label: "Windows",
        });
    }
    if lower.contains("macos") || lower.contains("darwin") || lower.ends_with(".dmg") {
        return Some(Platform {
            key: "macos",
            label: "macOS",
        });
    }
    None
}

fn channel_platforms(channel: &Value) -> Option<Vec<String>> {
    channel
        .get("platforms")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_ascii_lowercase())
                .collect()
        })
}

/// Render the zero-script portal from release assets (platform cards) with all
/// interpolated values HTML-escaped (OEM channel data is untrusted input).
fn portal_html(release: &Value, channel: &Value) -> String {
    let channel_id = release
        .get("channel_id")
        .and_then(Value::as_str)
        .or_else(|| channel.get("channel").and_then(Value::as_str))
        .unwrap_or("official");
    let version = release
        .get("client_version")
        .and_then(Value::as_str)
        .unwrap_or("0.1.0");
    let product = channel
        .get("product_name")
        .and_then(Value::as_str)
        .unwrap_or(channel_id);
    let powered_by = channel
        .get("powered_by")
        .and_then(Value::as_str)
        .unwrap_or("hide");
    let product = html_escape(product);
    let channel_label = html_escape(channel_id);
    let version_label = html_escape(version);
    let allowed = channel_platforms(channel);
    let mut cards = String::new();
    if let Some(assets) = release.get("assets").and_then(Value::as_array) {
        for asset in assets {
            let Some(name) = asset.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(platform) = platform_for_asset(name) else {
                continue;
            };
            if let Some(allowed) = &allowed {
                if !allowed.iter().any(|entry| entry == platform.key) {
                    continue;
                }
            }
            let sha = asset.get("sha256").and_then(Value::as_str).unwrap_or("");
            let size = asset.get("size").and_then(Value::as_u64).unwrap_or(0);
            cards.push_str(&format!(
                "<li class=\"card\"><a href=\"/api/updates/client/{}\">{} ({size} 字节)</a><br><small>sha256 <code>{}</code></small></li>",
                html_escape(name),
                platform.label,
                html_escape(sha),
            ));
        }
    }
    let assets_html = if cards.is_empty() {
        "<p>暂无可用安装包（CLIENT-RELEASE.json 未声明本渠道平台资产）。</p>".to_owned()
    } else {
        format!("<ul class=\"cards\">{cards}</ul>")
    };
    let footer = if powered_by == "show" {
        "<p><small>Powered by ZeoKeel</small></p>"
    } else {
        ""
    };
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><title>{product} 下载</title><style>body{{font-family:system-ui;margin:40px;max-width:720px}}.cards{{list-style:none;padding:0}}.card{{border:1px solid #ddd;border-radius:8px;padding:12px;margin:8px 0}}a{{color:#0b5fff}}</style></head><body><h1>{product} 客户端下载</h1><p>渠道：{channel_label} · 版本：{version_label}</p>{assets_html}<p><small>Windows 安装包未签名，请核对 sha256。企业版客户端只从此服务端更新。</small></p>{footer}</body></html>"
    )
}

/// `GET /` and `GET /portal` — zero-script download portal (`docs/09` §5).
async fn portal() -> Response {
    let release = release().unwrap_or_else(|_| json!({}));
    let channel = channel_config();
    let html = portal_html(&release, &channel);
    let mut response = html.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'",
        ),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

/// Read a release from an arbitrary dir (tests/tooling).
pub fn read_release(dir: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(dir.join("CLIENT-RELEASE.json"))
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    #[test]
    fn version_ordering_is_numeric() {
        assert!(version_lt("0.9.0", "0.10.0"));
        assert!(version_lt("0.1.0", "0.2.0"));
        assert!(!version_lt("0.1.0", "0.1.0"));
        assert!(!version_lt("0.2.0", "0.1.0"));
        assert!(!version_lt("not-a-version", "0.1.0"));
        assert!(!version_lt("1", "1.0.0"));
    }

    #[test]
    fn range_parsing() {
        assert!(matches!(
            parse_range("bytes=0-4", 10),
            ByteRange::Partial { start: 0, end: 4 }
        ));
        assert!(matches!(
            parse_range("bytes=5-", 10),
            ByteRange::Partial { start: 5, end: 9 }
        ));
        assert!(matches!(
            parse_range("bytes=-3", 10),
            ByteRange::Partial { start: 7, end: 9 }
        ));
        assert!(matches!(
            parse_range("bytes=5-100", 10),
            ByteRange::Partial { start: 5, end: 9 }
        ));
        assert!(matches!(
            parse_range("bytes=20-", 10),
            ByteRange::Unsatisfiable
        ));
        assert!(matches!(parse_range("bytes=0-1,3-4", 10), ByteRange::Full));
        assert!(matches!(parse_range("items=0-1", 10), ByteRange::Full));
        assert!(matches!(
            parse_range("bytes=0-4", 0),
            ByteRange::Unsatisfiable
        ));
        assert!(matches!(
            parse_range("bytes=-0", 10),
            ByteRange::Unsatisfiable
        ));
    }

    #[test]
    fn html_escaping_covers_oem_values() {
        let escaped = html_escape("<Acme \"Keel\" & 'Co'>");
        assert_eq!(escaped, "&lt;Acme &quot;Keel&quot; &amp; &#39;Co&#39;&gt;");
        let html = portal_html(
            &json!({"channel_id": "official", "client_version": "0.1.0", "assets": []}),
            &json!({"product_name": "<script>alert(1)</script>", "powered_by": "hide"}),
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn platform_cards_follow_release_assets_and_channel_narrowing() {
        let release = json!({
            "channel_id": "official",
            "client_version": "0.1.0",
            "assets": [
                {"name": "ZeoKeel_0.1.0_linux_x64.AppImage", "sha256": "aa", "size": 18},
                {"name": "ZeoKeel_0.1.0_windows_x64.exe", "sha256": "bb", "size": 20},
                {"name": "ZeoKeel_0.1.0_macos_arm64.dmg", "sha256": "cc", "size": 22}
            ]
        });
        let all = portal_html(&release, &json!({"product_name": "ZeoKeel"}));
        assert!(all.contains("linux_x64.AppImage"));
        assert!(all.contains("windows_x64.exe"));
        assert!(all.contains("macos_arm64.dmg"));
        assert!(all.contains("Linux"));
        assert!(all.contains("Windows"));
        assert!(all.contains("macOS"));

        let narrowed = portal_html(
            &release,
            &json!({"product_name": "ZeoKeel", "platforms": ["linux"]}),
        );
        assert!(narrowed.contains("linux_x64.AppImage"));
        assert!(!narrowed.contains("windows_x64.exe"));
        assert!(!narrowed.contains("macos_arm64.dmg"));

        let empty = portal_html(&json!({"assets": []}), &json!({}));
        assert!(empty.contains("暂无可用安装包"));
    }
    #[tokio::test]
    async fn manifest_below_min_returns_nested_426_envelope() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("CLIENT-RELEASE.json"),
            serde_json::json!({
                "schema": 1,
                "channel_id": "official",
                "edition": "enterprise",
                "server_version": "0.1.0",
                "client_version": "9.9.9",
                "min_supported": "9.9.9",
                "forced": false,
                "assets": []
            })
            .to_string(),
        )
        .expect("write release");
        let old = std::env::var_os("ZEOKEEL_UPDATE_DIR");
        std::env::set_var("ZEOKEEL_UPDATE_DIR", dir.path());
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/zk_never")
            .expect("lazy pool");
        let app = routes().with_state(AppState::new(pool));
        let request = Request::builder()
            .uri("/api/updates/manifest")
            .header("x-zeokeel-client", "0.5.0;official;linux-x64;test")
            .header("x-request-id", "req-updates-426")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        if let Some(value) = old {
            std::env::set_var("ZEOKEEL_UPDATE_DIR", value);
        } else {
            std::env::remove_var("ZEOKEEL_UPDATE_DIR");
        }
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        assert_eq!(
            response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("req-updates-426")
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["error"]["code"], "CLIENT_TOO_OLD");
        assert_eq!(value["error"]["retryable"], false);
        assert_eq!(value["error"]["request_id"], "req-updates-426");
        assert_eq!(value["error"]["current"], "0.5.0");
        assert_eq!(value["error"]["minimum"], "9.9.9");
        assert_eq!(value["error"]["portal_url"], "/portal");
    }
}
