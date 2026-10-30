//! UI declaration for `ui-settings`.
#![forbid(unsafe_code)]

use zk_contract_ui::UiContribution;

/// Static contributions mirrored from `zk.toml`.
#[must_use]
pub fn contributions() -> Vec<UiContribution> {
    vec![UiContribution {
        slot: "ui.settings".to_owned(),
        component: "settings.dialog".to_owned(),
        order: 10,
    }]
}
