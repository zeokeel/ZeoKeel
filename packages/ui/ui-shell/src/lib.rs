//! UI module declaration for `ui-shell`.
//!
//! This crate carries only the static contribution descriptor. The React
//! implementation lives in `web/` and is composed by `zk compose`.
#![forbid(unsafe_code)]

use zk_contract_ui::UiContribution;

/// Static contributions mirrored from `zk.toml`.
#[must_use]
pub fn contributions() -> Vec<UiContribution> {
    vec![UiContribution {
        slot: "ui.shell".to_owned(),
        component: "shell.frame".to_owned(),
        order: 10,
    }]
}
