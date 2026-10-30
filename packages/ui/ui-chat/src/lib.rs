//! UI declaration for `ui-chat`.
#![forbid(unsafe_code)]

use zk_contract_ui::UiContribution;

/// Static contributions mirrored from `zk.toml`.
#[must_use]
pub fn contributions() -> Vec<UiContribution> {
    vec![UiContribution {
        slot: "ui.chat".to_owned(),
        component: "chat.thread".to_owned(),
        order: 10,
    }]
}
