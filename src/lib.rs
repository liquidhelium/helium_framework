pub mod action;
pub mod hotkeys;
pub mod menu;
pub mod tab_system;
pub mod utils;
pub mod widgets;
pub mod notifications;

use action::ActionPlugin;
use bevy::app::Plugin;
use hotkeys::HotkeyPlugin;
use menu::MenuPlugin;
use notifications::NotificationPlugin;
use rust_i18n::i18n;
use tab_system::TabPlugin;
i18n!();

pub struct HeliumFramework;

impl Plugin for HeliumFramework {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((ActionPlugin, HotkeyPlugin, TabPlugin, MenuPlugin, NotificationPlugin));
    }
}

pub mod prelude {
    pub use super::{action::*, hotkeys::*, menu::*, tab_system::*, utils::*, notifications::*, HeliumFramework};
}
