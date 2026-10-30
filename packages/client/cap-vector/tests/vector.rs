#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)] // test-only

//! Vector capability tests with a deterministic in-process embedding model
//! (rig `Local<Embedding>` wire + a fake transport), so no network/provider is
//! needed and the full embed → insert → search path is exercised.

use std::sync::{Mutex, MutexGuard};

use futures::stream;
use rig_core::driver::{Exchange, Local, Model, Opened, Opening, Step, Transport};
use rig_core::embeddings::{Embedding as Vector, EmbeddingResponse};
use rig_core::operation::Embedding;
use rig_core::DynModel;
use zk_cap_vector::{VectorError, UNAVAILABLE};

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone)]
struct FakeEmbeddingTransport;

impl Transport<Local<Embedding>> for FakeEmbeddingTransport {
    fn send(&self, payload: Vec<String>, _exchange: Exchange) -> Opening<Step<Embedding>> {
        let vectors = payload
            .into_iter()
            .map(|text| Vector {
                document: text.clone(),
                vec: fake_vector(&text),
            })
            .collect();
        Opening::ready(Opened::new(stream::once(async move {
            Ok(Step::End(EmbeddingResponse::new(vectors)))
        })))
    }
}

fn fake_vector(text: &str) -> Vec<f64> {
    let alpha = f64::from(u8::from(text.contains("alpha")));
    let beta = f64::from(u8::from(text.contains("beta")));
    vec![alpha, beta, 1.0]
}

fn fake_model() -> DynModel<Embedding> {
    Model::new(
        Local::<Embedding>::new("fake-embed"),
        FakeEmbeddingTransport,
    )
    .into()
}

#[test]
fn unavailable_is_honest_and_explicit() {
    let _guard = guard();
    zk_cap_vector::unconfigure().unwrap();
    assert!(!zk_cap_vector::available());
    let reason = zk_cap_vector::unavailable_reason().expect("reason");
    assert!(reason.contains("no embedding model configured"), "{reason}");
    let error = zk_cap_vector::insert("doc", "alpha").unwrap_err();
    match error {
        VectorError::Unavailable(message) => {
            assert!(
                message.contains("no embedding model configured"),
                "{message}"
            );
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert_eq!(UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
}

#[test]
fn configured_capability_embeds_inserts_and_searches() {
    let _guard = guard();
    zk_cap_vector::configure(fake_model()).unwrap();
    zk_cap_vector::clear().unwrap();
    assert!(zk_cap_vector::available());
    assert_eq!(zk_cap_vector::unavailable_reason(), None);

    zk_cap_vector::insert("doc-alpha", "alpha document").unwrap();
    zk_cap_vector::insert("doc-beta", "beta document").unwrap();
    zk_cap_vector::insert_many([
        (
            "doc-alpha-2".to_owned(),
            "another alpha document".to_owned(),
        ),
        ("doc-gamma".to_owned(), "gamma document".to_owned()),
    ])
    .unwrap();
    assert_eq!(zk_cap_vector::len(), 4);

    let hits = zk_cap_vector::search("alpha", 2).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id, "doc-alpha");
    assert!(hits[0].score > 0.0);
    assert!(hits.iter().all(|hit| hit.text.contains("alpha")));

    // Inserting the same id replaces rather than duplicates.
    zk_cap_vector::insert("doc-alpha", "alpha document v2").unwrap();
    assert_eq!(zk_cap_vector::len(), 4);

    zk_cap_vector::unconfigure().unwrap();
    assert!(!zk_cap_vector::available());
    assert_eq!(zk_cap_vector::len(), 0);
}

#[test]
fn empty_index_and_zero_limit_are_safe() {
    let _guard = guard();
    zk_cap_vector::configure(fake_model()).unwrap();
    zk_cap_vector::clear().unwrap();
    assert!(zk_cap_vector::search("alpha", 3).unwrap().is_empty());
    assert!(zk_cap_vector::search("alpha", 0).unwrap().is_empty());
}

#[test]
fn configure_from_env_is_optional() {
    let _guard = guard();
    if std::env::var("ZEOKEEL_EMBEDDING_BASE_URL").is_ok()
        || std::env::var("ZEOKEEL_EMBEDDING_API_KEY").is_ok()
        || std::env::var("ZEOKEEL_EMBEDDING_MODEL").is_ok()
    {
        return;
    }
    zk_cap_vector::unconfigure().unwrap();
    assert!(!zk_cap_vector::configure_from_env().unwrap());
    assert!(!zk_cap_vector::available());
}
