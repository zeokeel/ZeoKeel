//! Real HTTP smoke: manifest ETag/304/426, Range/206, signature policy, and the
//! zero-script portal at `/` (+ `/portal`) with escaped OEM values.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)] // test-only

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use tower::ServiceExt;

const MANIFEST: &str = "/api/updates/manifest";
const CLIENT: &str = "x-zeokeel-client";

fn app() -> axum::Router {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/zk_never")
        .unwrap();
    zk_srv_updates::routes().with_state(zk_kit_server::AppState::new(pool))
}

async fn send(request: Request<Body>) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
    let response = app().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, headers, json)
}

async fn get_json(
    uri: &str,
    client_header: Option<&str>,
    extra: &[(&str, &str)],
) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
    let mut builder = Request::builder().uri(uri).method("GET");
    if let Some(client) = client_header {
        builder = builder.header(CLIENT, client);
    }
    for (name, value) in extra {
        builder = builder.header(*name, *value);
    }
    send(builder.body(Body::empty()).unwrap()).await
}

async fn get_raw(uri: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let response = app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, body.to_vec())
}

fn release_value() -> serde_json::Value {
    serde_json::json!({
        "schema": 1,
        "channel_id": "official",
        "edition": "enterprise",
        "server_version": "0.1.0",
        "client_version": "0.1.0",
        "min_supported": "0.1.0",
        "forced": false,
        "assets": [
            {"name": "ZeoKeel_0.1.0_linux_x64.AppImage", "sha256": "aa", "size": 18},
            {"name": "ZeoKeel_0.1.0_windows_x64.exe", "sha256": "bb", "size": 20},
            {"name": "ZeoKeel_0.1.0_macos_arm64.dmg", "sha256": "cc", "size": 22}
        ]
    })
}

fn write_release(dir: &std::path::Path, release: &serde_json::Value) {
    std::fs::write(
        dir.join("CLIENT-RELEASE.json"),
        serde_json::to_string_pretty(release).unwrap(),
    )
    .unwrap();
}

fn write_channel(dir: &std::path::Path, channel: &serde_json::Value) {
    std::fs::write(
        dir.join("CHANNEL.json"),
        serde_json::to_string(channel).unwrap(),
    )
    .unwrap();
}

fn write_clients(dir: &std::path::Path) {
    let clients = dir.join("clients");
    std::fs::create_dir_all(&clients).unwrap();
    std::fs::write(
        clients.join("ZeoKeel_0.1.0_linux_x64.AppImage"),
        b"0123456789abcdefgh",
    )
    .unwrap();
}

fn base64(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test]
async fn updates_http_contract_smoke() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let release = release_value();
    write_release(root, &release);
    write_channel(
        root,
        &serde_json::json!({
            "channel": "official",
            "product_name": "Acme <Keel> & \"Co\"",
            "powered_by": "show"
        }),
    );
    write_clients(root);
    std::env::set_var("ZEOKEEL_UPDATE_DIR", root);
    std::env::remove_var("ZEOKEEL_UPDATES_SIGNATURE_KEY");
    std::env::remove_var("ZEOKEEL_UPDATES_MIN_SUPPORTED");
    std::env::remove_var("ZEOKEEL_UPDATES_FORCED");

    // --- manifest: 200 + stable ETag, forced/min_supported, honest signature policy
    let (status, headers, manifest) =
        get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::OK, "{manifest}");
    let etag = headers
        .get("etag")
        .expect("ETag required by docs/04 §2.7")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(etag.starts_with('"') && etag.ends_with('"'), "{etag}");
    assert_eq!(manifest["forced"], false);
    assert_eq!(manifest["min_supported"], "0.1.0");
    assert_eq!(manifest["signature"], serde_json::Value::Null);
    assert_eq!(manifest["signature_status"], "unconfigured");
    assert_eq!(headers.get("x-zeokeel-signature").unwrap(), "unconfigured");

    // --- If-None-Match -> 304 with same ETag
    let (status, headers, body) = get_json(
        MANIFEST,
        Some("0.1.0;official;linux-x64"),
        &[("if-none-match", &etag)],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(headers.get("etag").unwrap().to_str().unwrap(), etag);
    assert_eq!(body, serde_json::Value::Null);

    // --- 426 below min_supported; 409 for a normal 1:1 mismatch
    let (status, _headers, body) = get_json(MANIFEST, Some("0.0.1;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED, "{body}");
    assert_eq!(body["error"]["code"], "CLIENT_TOO_OLD");
    assert_eq!(body["error"]["retryable"], false);
    assert!(body["error"]["request_id"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    assert_eq!(body["error"]["current"], "0.0.1");
    assert_eq!(body["error"]["minimum"], "0.1.0");
    assert_eq!(body["error"]["portal_url"], "/portal");
    let (status, _headers, body) = get_json(MANIFEST, Some("0.2.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["error"], "CLIENT_VERSION_MISMATCH");

    // --- forced from server setting (release metadata stays false)
    std::env::set_var("ZEOKEEL_UPDATES_FORCED", "1");
    let (status, _headers, manifest) =
        get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(manifest["forced"], true);
    std::env::remove_var("ZEOKEEL_UPDATES_FORCED");

    // --- download: full 200 + Accept-Ranges, 206 Range, 416 unsatisfiable
    let artifact = "/api/updates/client/ZeoKeel_0.1.0_linux_x64.AppImage";
    let (status, headers, body) = get_raw(artifact).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"0123456789abcdefgh");
    assert_eq!(headers.get("accept-ranges").unwrap(), "bytes");
    assert_eq!(headers.get("content-length").unwrap(), "18");

    let (status, headers, body) = {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri(artifact)
                    .header("range", "bytes=5-9")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, headers, body.to_vec())
    };
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body, b"56789");
    assert_eq!(headers.get("content-range").unwrap(), "bytes 5-9/18");
    assert_eq!(headers.get("accept-ranges").unwrap(), "bytes");

    let response = app()
        .oneshot(
            Request::builder()
                .uri(artifact)
                .header("range", "bytes=100-")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        response.headers().get("content-range").unwrap(),
        "bytes */18"
    );

    // --- portal at / and /portal: ignored OEM HTML is escaped, platform cards
    for uri in ["/", "/portal"] {
        let (status, headers, body) = get_raw(uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}");
        let html = String::from_utf8(body).unwrap();
        let csp = headers
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(csp.contains("default-src 'none'"), "{csp}");
        assert!(
            html.contains("Acme &lt;Keel&gt; &amp; &quot;Co&quot;"),
            "{html}"
        );
        assert!(
            !html.contains("<Keel>"),
            "unescaped OEM product leaked: {html}"
        );
        assert!(html.contains("linux_x64.AppImage"));
        assert!(html.contains("windows_x64.exe"));
        assert!(html.contains("macos_arm64.dmg"));
        assert!(html.contains("Powered by ZeoKeel"));
    }

    // --- channel platforms narrow the cards (unlisted block not rendered)
    write_channel(
        root,
        &serde_json::json!({
            "channel": "official",
            "product_name": "Acme <Keel>",
            "platforms": ["linux"]
        }),
    );
    let (status, _headers, body) = get_raw("/").await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("linux_x64.AppImage"));
    assert!(!html.contains("windows_x64.exe"));
    assert!(!html.contains("macos_arm64.dmg"));

    // --- ed25519 manifest signature policy
    let signing = SigningKey::from_bytes(&[7_u8; 32]);
    let public = signing.verifying_key();
    std::env::set_var("ZEOKEEL_UPDATES_SIGNATURE_KEY", base64(public.as_bytes()));
    let canonical = zk_srv_updates::canonical_manifest_bytes(&release).unwrap();
    let signature = base64(&signing.sign(&canonical).to_bytes());
    let mut signed = release.clone();
    signed["signature"] = serde_json::json!(signature);
    write_release(root, &signed);
    let (status, headers, manifest) =
        get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::OK, "{manifest}");
    assert_eq!(manifest["signature_status"], "verified");
    assert_eq!(manifest["signature"], signature);
    assert_eq!(headers.get("x-zeokeel-signature").unwrap(), "verified");

    // Tampered release -> fail closed.
    let mut tampered = signed.clone();
    tampered["forced"] = serde_json::json!(true);
    write_release(root, &tampered);
    let (status, _headers, body) = get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["error"], "UPDATE_SIGNATURE_INVALID");

    // Key configured but manifest unsigned -> fail closed.
    write_release(root, &release);
    let (status, _headers, body) = get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["error"], "UPDATE_SIGNATURE_REQUIRED");

    // No key configured but release is signed -> honest "unverified" (not "verified").
    std::env::remove_var("ZEOKEEL_UPDATES_SIGNATURE_KEY");
    write_release(root, &signed);
    let (status, _headers, manifest) =
        get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(manifest["signature_status"], "unverified");

    // --- missing release: portal still 200 (empty state), manifest 404
    std::fs::remove_file(root.join("CLIENT-RELEASE.json")).unwrap();
    let (status, _headers, body) = get_raw("/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(body).unwrap().contains("暂无可用安装包"));
    let (status, _headers, body) = get_json(MANIFEST, Some("0.1.0;official;linux-x64"), &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "UPDATE_UNAVAILABLE");

    std::env::remove_var("ZEOKEEL_UPDATE_DIR");
    std::env::remove_var("ZEOKEEL_UPDATES_SIGNATURE_KEY");
    std::env::remove_var("ZEOKEEL_UPDATES_MIN_SUPPORTED");
    std::env::remove_var("ZEOKEEL_UPDATES_FORCED");
}
