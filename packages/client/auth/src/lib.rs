//! Enterprise client auth against the login server (`auth.client`).
#![forbid(unsafe_code)]

use serde::Deserialize;
use serde_json::json;
use zk_contract_auth_client::AuthSession;

/// Auth client errors (closed).
#[derive(Debug)]
pub enum AuthClientError {
    /// Network/HTTP failure.
    Transport(String),
    /// Server returned an error code.
    Server(String),
    /// Malformed response.
    Protocol(String),
}

impl std::fmt::Display for AuthClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "auth transport: {error}"),
            Self::Server(code) => write!(f, "auth server: {code}"),
            Self::Protocol(error) => write!(f, "auth protocol: {error}"),
        }
    }
}
impl std::error::Error for AuthClientError {}

/// Client bound to one login server.
#[derive(Clone)]
pub struct AuthClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    user: User,
}
#[derive(Deserialize)]
struct User {
    id: String,
    role: String,
    username: String,
}
#[derive(Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

impl AuthClient {
    /// Create a client for a server base URL (e.g. `https://server.example.com`).
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_owned(), http: reqwest::Client::new() }
    }

    async fn post(&self, path: &str, body: serde_json::Value, bearer: Option<&str>) -> Result<reqwest::Response, AuthClientError> {
        let mut request = self.http.post(format!("{}{path}", self.base_url)).json(&body);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }
        request.send().await.map_err(|error| AuthClientError::Transport(error.to_string()))
    }

    async fn decode<T: for<'de> Deserialize<'de>>(response: reqwest::Response) -> Result<T, AuthClientError> {
        let status = response.status();
        let body = response.text().await.map_err(|error| AuthClientError::Transport(error.to_string()))?;
        if !status.is_success() {
            let code = serde_json::from_str::<serde_json::Value>(&body).ok().and_then(|value| value.get("error").and_then(|v| v.as_str()).map(str::to_owned)).unwrap_or_else(|| status.as_u16().to_string());
            return Err(AuthClientError::Server(code));
        }
        serde_json::from_str(&body).map_err(|error| AuthClientError::Protocol(error.to_string()))
    }

    /// Password login.
    pub async fn login(&self, username: &str, password: &str, device_id: &str) -> Result<AuthSession, AuthClientError> {
        let response = self.post("/api/client/v1/auth/login", json!({ "username": username, "password": password, "device_id": device_id }), None).await?;
        let body: LoginResponse = Self::decode(response).await?;
        Ok(AuthSession {
            access_token: body.access_token,
            refresh_token: body.refresh_token,
            expires_in: body.expires_in,
            user_id: body.user.id,
            role: body.user.role,
            username: body.user.username,
        })
    }

    /// Rotate tokens.
    pub async fn refresh(&self, session: &AuthSession) -> Result<AuthSession, AuthClientError> {
        let response = self.post("/api/client/v1/auth/refresh", json!({ "refresh_token": session.refresh_token, "device_id": "client" }), None).await?;
        let body: RefreshResponse = Self::decode(response).await?;
        Ok(AuthSession {
            access_token: body.access_token,
            refresh_token: body.refresh_token,
            expires_in: body.expires_in,
            user_id: session.user_id.clone(),
            role: session.role.clone(),
            username: session.username.clone(),
        })
    }

    /// Current user.
    pub async fn me(&self, access_token: &str) -> Result<serde_json::Value, AuthClientError> {
        let response = self.http.get(format!("{}/api/client/v1/auth/me", self.base_url)).bearer_auth(access_token).send().await.map_err(|error| AuthClientError::Transport(error.to_string()))?;
        Self::decode(response).await
    }

    /// Revoke refresh tokens.
    pub async fn logout(&self, access_token: &str) -> Result<(), AuthClientError> {
        let response = self.post("/api/client/v1/auth/logout", json!({}), Some(access_token)).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(AuthClientError::Server(status.as_u16().to_string()));
        }
        Ok(())
    }
}
