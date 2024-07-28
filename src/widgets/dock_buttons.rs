use bevy::prelude::*;

use crate::{tab_system::TabRegistry, prelude::HeDockstate};

pub fn dock_button(In(ui): In<&'static mut egui::Ui>, mut state: ResMut<HeDockstate>, registry: Res<TabRegistry>) {
    let state = &mut state.0;
    let opened: Vec<_> = state.iter_all_tabs().map(|i| i.1).collect();
    let mut to_remove = None;
    let mut to_add = None;
    for (i, tab) in registry.iter() {
        let is_opened = opened.contains(&i);
        if ui.selectable_label(is_opened, tab.title()).clicked() {
            if is_opened {
                to_remove = Some(state.find_tab(i).expect("i is opened but then not found?"));
                ui.close_menu();
            } else {
                to_add = Some(i.clone());
                ui.close_menu();
            }
        }
    }
    if let Some(to) = to_remove {
        state.remove_tab(to);
    }
    if let Some(to) = to_add {
        state.add_window(vec![to]);
    }
}
