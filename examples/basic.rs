use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiPlugin};
use egui::Ui;
use egui_dock::{DockArea, DockState};
use helium_framework::{
    menu::{show_menu_ui, Button, Custom, MenuExt},
    prelude::*,
    tab_system::{HeDockState, HeTabViewer, TabRegistrationExt, TabRegistry},
    widgets::{dock_button, widget},
};
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_plugins(HeliumFramework)
        .insert_resource(HeDockState(DockState::new(vec!["default".into()])));
    app.add_event::<ButtonClicked>();
    app.register_action("maximize", "show mouse, events", it_works)
        .register_action("basic.log_clicked", "log click times", log_button_clicked)
        .register_action("quit", "quit", || std::process::exit(0));
    app.register_tab("default", "Default", default_tab, || true)
        .register_tab("default2", "Default2", default_tab, || true)
        .register_tab("default3", "Default3", default_tab, || true)
        .register_tab("default4", "Default4", default_tab, || true)
        .register_tab("default5", "Default5", default_tab, || true);
    app.register_hotkey("maximize", [Hotkey::new_global([KeyCode::ControlLeft, KeyCode::KeyM])]);
    app.menu_context(|mut ctx| {
        ctx.with_sub_menu("file", "File".into(), 0, |mut ctx| {
            ctx.add("quit", "Quit".into(), Button::new("quit"), 0);
        });
        ctx.with_sub_menu("window", "Window".into(), 1, |mut ctx| {
            ctx.add(
                "win",
                "".into(),
                Custom(Box::new(|ui, world, _| widget(world,ui,dock_button))),
                0,
            );
        });
    });
    app.add_systems(Update, egui_main);
    app.run();
}
#[derive(Event)]
struct ButtonClicked;

fn it_works(mut windows: Query<&mut Window>) {
    windows.par_iter_mut().for_each(|mut win| {
        win.set_maximized(true);
    });
}
fn log_button_clicked(clickbutton: EventReader<ButtonClicked>) {
    info!("{}", clickbutton.len());
}

fn default_tab(
    In(mut ui): In<Ui>,
    mut clickbutton: EventWriter<ButtonClicked>,
    mut action: Actions,
) {
    ui.heading("Helium Framework test");
    ui.label("This one works!");
    if ui.button("Click this to maximize the window").clicked() {
        action.run_action(&"maximize".into(), ()).unwrap();
    }
    if ui
        .button("click this to log how many times this has been clicked")
        .clicked()
    {
        clickbutton.send(ButtonClicked);
        action.run_action(&"basic.log_clicked".into(), ()).unwrap();
    }
}

fn egui_main(world: &mut World) {
    let mut egui_context = world.query_filtered::<&mut EguiContext, With<PrimaryWindow>>();
    let mut binding = egui_context.single_mut(world);
    let ctx = &binding.get_mut().clone();
    egui::TopBottomPanel::top("menu").show(ctx, |ui| {
        ui.horizontal(|ui| {
            show_menu_ui(ui, world);
            ui.label("Press ctrl+m to trigger hotkey")
        });
    });
    world.resource_scope(|world: &mut World, mut registry: Mut<'_, TabRegistry>| {
        world.resource_scope(|world: &mut World, mut state: Mut<'_, HeDockState>| {
            DockArea::new(&mut state.0).show(
                ctx,
                &mut HeTabViewer {
                    registry: &mut registry,
                    world,
                },
            );
        })
    })
}
