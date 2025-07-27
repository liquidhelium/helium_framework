use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{EguiContext, EguiPlugin};
use egui::{style::Selection, Color32, Ui, Visuals};
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
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: false,
        })
        .add_plugins(HeliumFramework)
        .insert_resource(HeDockState(DockState::new(vec!["default".into()])));
    app.add_event::<ButtonClicked>();
    app.reflect_system("maximize", "show mouse, events", it_works)
        .reflect_system("basic.log_clicked", "log click times", log_button_clicked)
        .reflect_system("basic.log_current", "log current time", log_current)
        .reflect_system(
            "basic.log_reference",
            "log current time by reference",
            log_reference,
        )
        // Explict type parameter to make the compiler happy
        .reflect_system::<_, _, ()>("quit", "quit", || {
            std::process::exit(0);
        });
    app.register_tab("default", "Default", default_tab, || true)
        .register_tab("default2", "Default2", default_tab, || true)
        .register_tab("default3", "Default3", default_tab, || true)
        .register_tab("default4", "Default4", default_tab, || true)
        .register_tab("default5", "Default5", default_tab, || true)
        .register_tab("another", "Another", another_tab, || true);
    app.register_hotkey(
        "maximize",
        [Hotkey::new_global([KeyCode::ControlLeft, KeyCode::KeyM])],
    );
    app.menu_context(|mut ctx| {
        ctx.with_sub_menu("file", "File".into(), 0, |mut ctx| {
            ctx.add("quit", "Quit".into(), Button::new("quit"), 0);
        });
        ctx.with_sub_menu("window", "Window".into(), 1, |mut ctx| {
            ctx.add(
                "win",
                "".into(),
                Custom(Box::new(|ui, world, _| widget(world, ui, dock_button))),
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

fn log_current(In(time): In<f32>) {
    info!("Current time: {}", time);
}

fn log_reference(InMut(time): InMut<f32>) {
    info!("Current time: {}", time);
    *time += 1.0; // just to show that we can modify the value
}

fn log_button_clicked(clickbutton: EventReader<ButtonClicked>, mut count: Local<usize>) {
    *count += clickbutton.len();
    info!("{}", *count);
}

fn default_tab(
    InMut(ui): InMut<Ui>,
    mut clickbutton: EventWriter<ButtonClicked>,
    mut action: Actions,
    action_registry: Res<RSystemRegistry>,
    time: Res<Time>,
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
    if ui.button("Log current time").clicked() {
        action
            .run_action(&"basic.log_current".into(), In(time.elapsed_secs()))
            .unwrap();
    }
    // list all registered actions and in/outputs
    ui.label("Registered actions:");
    for (id, meta) in action_registry.iter() {
        ui.label(format!(
            "Action: {}, Inputs: {:?}, Outputs: {:?}",
            id, meta.input, meta.output
        ));
    }
}

fn another_tab(InMut(ui): InMut<Ui>, world: &mut World) {
    ui.heading("Another tab");
    ui.label("This is another tab.");
    if ui.button("click this to log time(by reference)").clicked() {
        let elapsed_secs = &mut world.resource::<Time>().elapsed_secs();
        world.resource_scope(|world: &mut World, mut actions: Mut<'_, RSystemRegistry>| {
            actions
                .run_instant(&"basic.log_reference".into(), InMut(elapsed_secs), world)
                .unwrap();
        });
        info!("time added 1.0, time: {}", elapsed_secs);
    }
}

fn egui_main(world: &mut World) -> Result<()> {
    let mut egui_context = world.query_filtered::<&mut EguiContext, With<PrimaryWindow>>();
    let mut binding = egui_context.single_mut(world)?;
    let ctx = &binding.get_mut().clone();
    ctx.set_visuals(Visuals {
        dark_mode: true,
        extreme_bg_color: rgba(23, 23, 23, 1.0),
        widgets: egui::style::Widgets {
            inactive: egui::style::WidgetVisuals {
                weak_bg_fill: rgba(50, 50, 50, 0.0),
                bg_stroke: egui::Stroke::new(1.0, rgba(200, 200, 200, 0.0)),

                ..Visuals::dark().widgets.active
            },
            hovered: egui::style::WidgetVisuals {
                weak_bg_fill: rgba(100,100,100, 0.4),
                bg_stroke: egui::Stroke::new(1.0, rgba(200, 200, 200, 0.0)),

                ..Visuals::dark().widgets.active
            },
            ..Default::default()
        },
        ..Visuals::dark()
    });
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
    });
    Ok(())
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, (a * 255.0) as u8)
}
