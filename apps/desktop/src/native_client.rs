//! Native (Tauri) HTTP client identity.
//!
//! Native app-window requests bypass the browser `serverFetch` wrapper, so they
//! must attach the same `X-ZeoKeel-Client`/`X-Request-Id` identity the server
//! business gate expects (`docs/04` §2.2/§2.7, round-6 P1-1).

/// Header carrying the native client identity.
pub(crate) const CLIENT_HEADER: &str = "x-zeokeel-client";
/// Header carrying the correlation id echoed by the server.
pub(crate) const REQUEST_ID_HEADER: &str = "x-request-id";

fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn device_id() -> String {
    let raw = std::env::var("ZEOKEEL_DEVICE_ID").unwrap_or_default();
    let sanitized: String = raw
        .chars()
        .filter(|value| !value.is_control() && *value != ';')
        .take(64)
        .collect();
    if sanitized.is_empty() {
        "desktop".to_owned()
    } else {
        sanitized
    }
}

/// `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device_id>`.
#[must_use]
pub(crate) fn client_header_value() -> String {
    format!(
        "{};{};{};{}",
        env!("CARGO_PKG_VERSION"),
        zk_composed::CHANNEL,
        platform(),
        device_id()
    )
}

/// Fresh request id (server echoes it in `X-Request-Id` and 426 bodies).
#[must_use]
pub(crate) fn request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Attach native identity + correlation headers to a blocking request.
pub(crate) fn apply_headers(
    builder: reqwest::blocking::RequestBuilder,
) -> reqwest::blocking::RequestBuilder {
    builder
        .header(CLIENT_HEADER, client_header_value())
        .header(REQUEST_ID_HEADER, request_id())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

    #[test]
    fn client_header_has_version_channel_platform_device() {
        let header = super::client_header_value();
        let parts: Vec<&str> = header.split(';').collect();
        assert_eq!(parts.len(), 4, "{header}");
        assert_eq!(parts[0], env!("CARGO_PKG_VERSION"));
        assert_eq!(parts[1], zk_composed::CHANNEL);
        assert_eq!(
            parts[2],
            format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
        );
        assert!(!parts[3].is_empty());
    }

    #[test]
    fn apply_headers_sets_client_and_request_id() {
        let client = reqwest::blocking::Client::new();
        let request = super::apply_headers(client.get("http://127.0.0.1:1/health"))
            .build()
            .unwrap();
        let headers = request.headers();
        let value = headers
            .get(super::CLIENT_HEADER)
            .and_then(|value| value.to_str().ok())
            .expect("client header");
        assert_eq!(value.split(';').count(), 4);
        let request_id = headers
            .get(super::REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .expect("request id");
        assert!(uuid::Uuid::parse_str(request_id).is_ok(), "{request_id}");
    }
}
