use std::{any::{Any, TypeId}, borrow::Cow, intrinsics::type_id};

use bevy::{ecs::{schedule::BoxedCondition, system::SystemId}, prelude::*, reflect::Typed, utils::HashMap};
use egui::{Ui, UiBuilder};
use egui_dock::{DockState, TabViewer};
use rust_i18n::t;
use snafu::Snafu;

use crate::{reflect_system::ReflectSystemId, utils::{identifier::Identifier, new_condition}};

pub struct HeTabViewer<'a> {
    pub world: &'a mut World,
    pub registry: &'a mut TabRegistry,
}

impl<'a> TabViewer for HeTabViewer<'a> {
    type Tab = TabId;
    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        self.registry
            .get(tab)
            .map(|t| t.title())
            .unwrap_or("MISSINGNO".into())
            .into()
    }
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        self.registry.tab_ui(ui, self.world, tab);
    }
}

#[derive(Debug, Resource)]
pub struct HeDockState(pub DockState<TabId>);

pub type TabId = Identifier;

pub struct TabStorage {
    system_id: ReflectSystemId,
    avalible_condition: BoxedCondition,
    tab_title: Cow<'static, str>,
}

#[derive(Resource, Default, PartialEq, Eq)]
pub struct FocusedTab(pub Option<TabId>);

pub fn tab_focused(tab: impl Into<TabId>) -> impl Condition<()> {
    resource_exists_and_equals(FocusedTab(Some(tab.into()))).and(|| true)
}

pub fn tab_opened(tab: impl Into<TabId>) -> impl Condition<()> {
    let tab = tab.into();
    (move |res: Option<Res<HeDockState>>| res.is_some_and(|res| res.0.find_tab(&tab).is_some()))
        .and(|| true)
}

impl TabStorage {
    pub fn run_with(&mut self, world: &mut World, ui: &mut Ui) -> TabResult {
        let child = {
            let max_rect = ui.max_rect();
            let layout = *ui.layout();
            ui.new_child(
                UiBuilder::new()
                    .max_rect(max_rect)
                    .layout(layout)
            )
        };
        let system_id = self.system_id.system_id::<In<Ui>, ()>().
            ok_or(TabError::InvalidType  {
                name: self.tab_title.clone(),
            })?;
        self.avalible_condition
            .run_readonly((), world)
            .then(|| {
                world.run_system_with_input(system_id, child);
            })
            .ok_or(TabError::NotAvalible {
                name: self.tab_title.clone(),
            })
    }
    pub fn title(&self) -> Cow<'static, str> {
        self.tab_title.clone()
    }
}
pub type TabResult = Result<(), TabError>;

#[derive(Snafu, Debug)]
pub enum TabError {
    #[snafu(display("Tab {name} is not avalible."))]
    NotAvalible { name: Cow<'static, str> },
    #[snafu(display("Tab {name} is invalid."))]
    InvalidType {
        name: Cow<'static, str>,
    }
}

#[derive(Resource, Deref, Default)]
pub struct TabRegistry(HashMap<TabId, TabStorage>);

impl TabRegistry {
    pub fn tab_ui(&mut self, ui: &mut Ui, world: &mut World, tab: &TabId) {
        use egui::{Color32, RichText};

        if let Some(tab) = self.0.get_mut(tab) {
            let Ok(()) = tab.run_with(world, ui) else {
                ui.colored_label(
                    Color32::GRAY,
                    RichText::new(t!("tab.not_avalible")).italics(),
                );
                return;
            };
        } else {
            ui.colored_label(Color32::RED, t!("tab.non_exist", tab = tab));
        }
    }
}

pub trait TabRegistrationExt {
    fn register_tab<M1, M2>(
        &mut self,
        id: impl Into<TabId>,
        name: impl Into<Cow<'static, str>>,
        system: impl IntoSystem<In<Ui>, (), M1> + 'static,
        avalible_when: impl Condition<M2>,
    ) -> &mut Self;
}

impl TabRegistrationExt for App {
    fn register_tab<M1, M2,>(
        &mut self,
        id: impl Into<TabId>,
        name: impl Into<Cow<'static, str>>,
        system: impl IntoSystem<In<Ui>, (), M1> +'static,
        avalible_when: impl Condition<M2>,
    ) -> &mut Self {
        self.world_mut()
            .resource_scope(|world, mut registry: Mut<TabRegistry>| {
                let system_id: bevy::ecs::system::SystemId<_, _> = world.register_system(system);
                let system_id = ReflectSystemId::from_system_id(system_id);
                registry.0.insert(
                    id.into(),
                    TabStorage {
                        system_id,
                        avalible_condition: {
                            let mut sys = new_condition(avalible_when);
                            sys.initialize(world);
                            sys
                        },
                        tab_title: name.into(),
                    },
                )
            });
        self
    }
}
pub struct TabPlugin;

impl Plugin for TabPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TabRegistry>()
            .init_resource::<FocusedTab>();
    }
}
