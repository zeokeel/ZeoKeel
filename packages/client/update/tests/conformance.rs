#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use axum::{routing::get, Json, Router};
use serde_json::json;
use zk_client_update_github::check;

#[test]
fn latest_json_selects_platform_asset() {
    let app = Router::new().route(
        "/latest.json",
        get(|| async {
            Json(json!({
                "version": "0.2.0",
                "notes": "test",
                "assets": [{ "platform": "linux-x86_64", "url": "https://example.invalid/app.tar.gz", "sha256": "abc" }]
            }))
        }),
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    std::env::set_var("ZEOKEEL_UPDATE_FEED", format!("http://{addr}/latest.json"));
    let result = check("0.1.0", "linux-x86_64").unwrap();
    assert!(result.available);
    assert_eq!(result.release.as_ref().unwrap().version, "0.2.0");
    assert!(result.asset_for("linux-x86_64").is_some());
    let same = check("0.2.0", "linux-x86_64").unwrap();
    assert!(!same.available);
}
