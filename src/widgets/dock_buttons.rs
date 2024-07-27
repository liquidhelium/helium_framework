use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::{tab_system::TabRegistry, prelude::HeDockstate};

use super::WidgetSystem;

#[derive(SystemParam)]
pub struct DockButtons<'w> {
    state: ResMut<'w, HeDockstate>,
    // tabs: Res<'w, RizTabs>,
    registry: Res<'w, TabRegistry>,
}

impl WidgetSystem for DockButtons<'static> {
    type Extra<'a> = ();
    fn system(
        world: &mut bevy::prelude::World,
        state: &mut bevy::ecs::system::SystemState<Self>,
        ui: &mut egui::Ui,
        _extra: Self::Extra<'_>,
    ) {
        let DockButtons::<'_> {
            mut state,
            registry,
        } = state.get_mut(world);
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
}
