#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use zk_client_auth::AuthClient;

async fn fake_server() -> String {
    let app = Router::new()
        .route("/api/client/v1/auth/login", post(|Json(_body): Json<Value>| async { Json(json!({
            "ok": true, "access_token": "a1", "refresh_token": "r1", "expires_in": 900,
            "user": { "id": "u1", "role": "employee", "username": "alice" }
        })) }))
        .route("/api/client/v1/auth/refresh", post(|Json(_body): Json<Value>| async { Json(json!({
            "ok": true, "access_token": "a2", "refresh_token": "r2", "expires_in": 900
        })) }))
        .route("/api/client/v1/auth/me", get(|| async { Json(json!({ "id": "u1", "role": "employee" })) }))
        .route("/api/client/v1/auth/logout", post(|| async { Json(json!({ "ok": true })) }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

#[tokio::test]
async fn login_refresh_me_logout() {
    let base = fake_server().await;
    let client = AuthClient::new(&base);
    let session = client.login("alice", "pw", "dev1").await.unwrap();
    assert_eq!(session.access_token, "a1");
    assert_eq!(session.user_id, "u1");
    let refreshed = client.refresh(&session).await.unwrap();
    assert_eq!(refreshed.access_token, "a2");
    let me = client.me(&refreshed.access_token).await.unwrap();
    assert_eq!(me["role"], "employee");
    client.logout(&refreshed.access_token).await.unwrap();
}
