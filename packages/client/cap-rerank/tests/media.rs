#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Rerank capability tests with a deterministic in-process model (rig
//! `Local<Rerank>` wire + fake transport): query/documents reach the model and
//! ranked scores come back without any network or provider.

use std::sync::{Mutex, MutexGuard};

use futures::stream;
use rig_core::driver::{Exchange, Local, Model, Opened, Opening, Step, Transport};
use rig_core::operation::{Rerank, RerankRequest};
use rig_core::rerank::{RerankResponse, RerankResult};
use rig_core::DynModel;
use zk_cap_rerank::{RerankError, RerankHit};

static LOCK: Mutex<()> = Mutex::new(());

fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone)]
struct FakeRerankTransport;

impl Transport<Local<Rerank>> for FakeRerankTransport {
    fn send(&self, payload: RerankRequest, _exchange: Exchange) -> Opening<Step<Rerank>> {
        let mut results: Vec<RerankResult> = payload
            .documents
            .iter()
            .enumerate()
            .map(|(index, document)| RerankResult {
                index,
                document: Some(document.clone()),
                relevance_score: if document.contains(&payload.query) {
                    1.0
                } else {
                    0.0
                },
            })
            .collect();
        results.sort_by(|left, right| right.relevance_score.total_cmp(&left.relevance_score));
        let mut response = RerankResponse::new(results);
        response.model = Some("fake-rerank-model".to_owned());
        Opening::ready(Opened::new(stream::once(
            async move { Ok(Step::End(response)) },
        )))
    }
}

fn fake_model() -> DynModel<Rerank> {
    Model::new(Local::<Rerank>::new("fake-provider"), FakeRerankTransport).into()
}

#[test]
fn configured_capability_reranks_documents() {
    let _guard = guard();
    zk_cap_rerank::configure(fake_model()).unwrap();
    assert!(zk_cap_rerank::available());
    assert_eq!(zk_cap_rerank::unavailable_reason(), None);

    let documents = vec![
        "python is slower".to_owned(),
        "rust is fast".to_owned(),
        "another rust document".to_owned(),
    ];
    let hits: Vec<RerankHit> = zk_cap_rerank::rerank("rust", &documents).unwrap();
    assert_eq!(hits.len(), 3);
    // Highest relevance first; the fake scores 1.0 for documents containing
    // the query and 0.0 otherwise.
    assert_eq!(hits.first().map(|hit| hit.index), Some(1));
    assert_eq!(hits.first().map(|hit| hit.score), Some(1.0));
    assert_eq!(hits.get(2).map(|hit| hit.score), Some(0.0));
    assert!(hits.iter().all(|hit| hit.document.is_some()));

    zk_cap_rerank::unconfigure().unwrap();
    assert!(!zk_cap_rerank::available());
    assert!(zk_cap_rerank::unavailable_reason().is_some());
}

#[test]
fn empty_query_is_rejected_and_empty_documents_short_circuit() {
    let _guard = guard();
    zk_cap_rerank::unconfigure().unwrap();
    // Empty document list needs no provider and returns no hits.
    assert!(zk_cap_rerank::rerank("query", &[]).unwrap().is_empty());
    match zk_cap_rerank::rerank("   ", &["one".to_owned()]).unwrap_err() {
        RerankError::Config(message) => assert!(message.contains("query is empty"), "{message}"),
        other => panic!("expected Config, got {other:?}"),
    }
}

#[test]
fn configure_from_env_is_optional() {
    let _guard = guard();
    if std::env::var("ZEOKEEL_RERANK_API_KEY").is_ok() {
        return;
    }
    zk_cap_rerank::unconfigure().unwrap();
    assert!(!zk_cap_rerank::configure_from_env().unwrap());
    assert!(!zk_cap_rerank::available());
}

#[test]
fn configure_from_config_rejects_openai_rerank() {
    let _guard = guard();
    let config = zk_llm_rig::MediaConfig::new("openai", "http://127.0.0.1:1/v1", "test-key", "");
    match zk_cap_rerank::configure_from_config(&config).unwrap_err() {
        RerankError::Config(message) => {
            assert!(message.contains("no rerank endpoint"), "{message}");
        }
        other => panic!("expected Config, got {other:?}"),
    }
    assert!(!zk_cap_rerank::available());
}
