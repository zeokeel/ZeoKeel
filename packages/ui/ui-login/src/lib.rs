//! UI contribution descriptor for the enterprise login page.
#![forbid(unsafe_code)]
use zk_contract_ui::UiContribution;

/// Static contribution mirrored from `zk.toml`.
#[must_use]
pub fn contributions() -> Vec<UiContribution> {
    vec![UiContribution {
        slot: "ui.login".to_owned(),
        component: "login.page".to_owned(),
        order: 10,
    }]
}
