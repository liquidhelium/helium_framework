mod dock_buttons;
pub use dock_buttons::*;

use bevy::prelude::*;
use egui::Ui;

pub fn widget<M: 'static, S: IntoSystem<&'static mut Ui, (), M> + 'static>(
    world: &mut World,
    ui: &mut Ui,
    wdg: S,
) {
    if !world.contains_resource::<CachedWidgetState<M, S>>() {
        let mut system = IntoSystem::into_system(wdg);
        system.initialize(world);
        let value = CachedWidgetState::<M, S>(system);
        world.insert_resource(value);
    }
    world.resource_scope(
        |world: &mut World, mut cache: Mut<'_, CachedWidgetState<M, S>>| {
            cache.0.run(unsafe { &mut *(ui as *mut Ui) }, world);
            cache.0.apply_deferred(world);
        },
    );
}

#[derive(Resource)]
struct CachedWidgetState<M, S: IntoSystem<&'static mut Ui, (), M> + 'static>(S::System);
