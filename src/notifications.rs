use bevy::{
    app::{Plugin, Update},
    ecs::{
        query::With,
        system::{Query, ResMut, Resource},
    },
    prelude::{Deref, DerefMut},
    window::PrimaryWindow,
};
use bevy_egui::EguiContext;
use egui_notify::Toasts;

pub struct NotificationPlugin;

impl Plugin for NotificationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<ToastsStorage>()
            .add_systems(Update, show_egui_notifies);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct ToastsStorage(Toasts);

impl Default for ToastsStorage {
    fn default() -> Self {
        Self(
            Toasts::new()
                .with_anchor(egui_notify::Anchor::BottomRight)
                .with_margin([8.0, 48.0].into()),
        )
    }
}

fn show_egui_notifies(
    mut context: Query<&mut EguiContext, With<PrimaryWindow>>,
    mut toasts: ResMut<ToastsStorage>,
) {
    if let Ok(mut ctx) = context.get_single_mut() {
        toasts.show(ctx.get_mut())
    }
}
