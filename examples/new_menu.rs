use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiPlugin};
use egui::{Label, Sense, Ui};
use egui_dock::DockState;
use helium_framework::{menu_system::*, prelude::*, tab_system::HeDockState};

#[derive(Debug)]
struct MainMenuContext;

#[derive(Debug)]
struct EditorContext;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        .add_plugins(HeliumFramework)
        .insert_resource(HeDockState(DockState::new(vec!["default".into()])));

    // Register menus using new system
    app.register_submenu::<MainMenuContext>("file", "file", "File")
    .register_command::<MainMenuContext>("file/new", "file.new", "New", "file.new")
        .register_command::<MainMenuContext>("file/quit", "file.quit", "Quit", "file.quit")
        .register_custom::<EditorContext>(
            "format",
            "editor.format",
            "Format Code",
            |ui, world, _ctx| {
                if ui.button("Format Code").clicked() {
                    info!("Formatting code...");
                }
            },
        )
        .register_command::<EditorContext>(
            "copy",
            "editor.copy",
            "Copy",
            "editor.copy",
        );

    app.add_systems(Update, egui_main);
    app.run();
}

fn egui_main(world: &mut World) {
    let mut egui_context = world.query_filtered::<&mut EguiContext, With<PrimaryWindow>>();
    let mut binding = egui_context.single_mut(world).unwrap();
    let ctx = &binding.get_mut().clone();
    
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            world.resource_scope(|world, mut menu_system: Mut<MenuSystem>| {
                // Show main menu bar
                menu_system.show_menu::<MainMenuContext>(ui, world, &MainMenuContext);
            });
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("New Menu System Demo");
        ui.add(Label::new("context").sense(Sense::all())).context_menu(
            |ui| {
                world.resource_scope(|world, mut menu_system: Mut<MenuSystem>| {
                    menu_system.show_menu::<EditorContext>(ui, world, &EditorContext);
                });
            }
        );
    });
}
