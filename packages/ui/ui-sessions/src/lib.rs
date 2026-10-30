//! UI declaration for `ui-sessions`.
#![forbid(unsafe_code)]

use zk_contract_ui::UiContribution;

/// Static contributions mirrored from `zk.toml`.
#[must_use]
pub fn contributions() -> Vec<UiContribution> {
    vec![UiContribution {
        slot: "ui.sessions".to_owned(),
        component: "sessions.sidebar".to_owned(),
        order: 10,
    }]
}
