#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_ui::UiContribution;

#[test]
fn contribution_round_trips() {
    let c = UiContribution { slot: "ui.chat".to_owned(), component: "chat.thread".to_owned(), order: 10 };
    let json = serde_json::to_string(&c).unwrap();
    assert_eq!(c, serde_json::from_str::<UiContribution>(&json).unwrap());
}
