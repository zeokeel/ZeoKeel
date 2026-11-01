#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_tool::{Tool, TurnContext};
use zk_tool_plan::{plans, ExitPlanModeTool, MAX_PLAN_BYTES};

fn context(root: &str) -> TurnContext {
    TurnContext {
        conversation_id: "c1".to_owned(),
        workspace_id: "w1".to_owned(),
        workspace_root: root.to_owned(),
    }
}

fn root() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("zk-plan-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn exit_plan_mode_requires_active_mode_and_submits_pending_review() {
    let dir = root();
    let root = dir.to_string_lossy().to_string();
    // Not active → plan/not-active (no file write).
    let tool = ExitPlanModeTool;
    let error = tool
        .call_json(&context(&root), serde_json::json!({ "plan": "# plan" }))
        .expect_err("inactive");
    assert!(format!("{error}").contains("plan/not-active"));
    // Enter plan mode, then submit.
    let mut file = plans::load(&root).unwrap();
    file.enter("c1", plans::now_ms());
    plans::save(&root, &file).unwrap();
    let out = tool
        .call_json(
            &context(&root),
            serde_json::json!({ "plan": "# plan\n- step" }),
        )
        .unwrap();
    assert!(out.contains("pending_review"));
    let loaded = plans::load(&root).unwrap();
    let entry = loaded.conversations.get("c1").unwrap();
    assert_eq!(entry.status, plans::PlanStatus::PendingReview);
    assert_eq!(entry.mode, plans::PlanMode::Active);
    assert_eq!(entry.plan, "# plan\n- step");
    // Invalid arguments and oversize plans are rejected without changing the store.
    assert!(tool
        .call_json(&context(&root), serde_json::json!({ "plan": "  " }))
        .is_err());
    let big = "x".repeat(MAX_PLAN_BYTES + 1);
    assert!(tool
        .call_json(&context(&root), serde_json::json!({ "plan": big }))
        .is_err());
    assert_eq!(
        plans::load(&root)
            .unwrap()
            .conversations
            .get("c1")
            .unwrap()
            .plan,
        "# plan\n- step"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn review_transitions_and_bounds() {
    let dir = root();
    let root = dir.to_string_lossy().to_string();
    let mut file = plans::PlanFile::default();
    file.enter("c1", 1);
    file.submit("c1", "# plan".to_owned(), 2).unwrap();
    file.reject("c1", Some("add tests".to_owned()), 3);
    let entry = file.conversations.get("c1").unwrap();
    assert_eq!(entry.status, plans::PlanStatus::Rejected);
    assert_eq!(entry.mode, plans::PlanMode::Active);
    assert!(entry.feedback.as_deref() == Some("add tests"));
    file.continue_editing("c1", 4);
    assert_eq!(
        file.conversations.get("c1").unwrap().status,
        plans::PlanStatus::Draft
    );
    file.submit("c1", "# plan v2".to_owned(), 5).unwrap();
    file.approve("c1", 6);
    let entry = file.conversations.get("c1").unwrap();
    assert_eq!(entry.status, plans::PlanStatus::Approved);
    assert_eq!(entry.mode, plans::PlanMode::Off);
    // File-level 64 KiB bound.
    let mut huge = plans::PlanFile::default();
    for index in 0..3 {
        let conversation = format!("c{index}");
        huge.enter(&conversation, 1);
        huge.submit(&conversation, "y".repeat(30 * 1024), 2)
            .unwrap();
    }
    assert!(matches!(
        plans::save(&root, &huge),
        Err(plans::PlanError::TooLarge)
    ));
    let _ = std::fs::remove_dir_all(&dir);
}
