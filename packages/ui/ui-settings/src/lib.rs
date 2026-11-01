//! UI declaration for `ui-settings`.
#![forbid(unsafe_code)]

use zk_contract_ui::UiContribution;

/// Static contributions mirrored from `zk.toml`.
///
/// The dialog shell plus the built-in `ui.settings.pane` contributions (docs/16
/// §2.4). `editions` is manifest-only (`zk-contract-ui::UiContribution` has no
/// such field); `zk compose` reads `zk.toml` directly for the build-time filter.
#[must_use]
pub fn contributions() -> Vec<UiContribution> {
    vec![
        UiContribution {
            slot: "ui.settings".to_owned(),
            component: "settings.dialog".to_owned(),
            order: 10,
        },
        pane("settings.pane.general", 10),
        pane("settings.pane.models", 20),
        pane("settings.pane.agent", 30),
        pane("settings.pane.tools", 40),
        pane("settings.pane.search", 50),
        pane("settings.pane.embedding", 60),
        pane("settings.pane.media", 65),
        pane("settings.pane.enterprise", 80),
        pane("settings.pane.about", 90),
    ]
}

fn pane(component: &str, order: i32) -> UiContribution {
    UiContribution {
        slot: "ui.settings.pane".to_owned(),
        component: component.to_owned(),
        order,
    }
}
