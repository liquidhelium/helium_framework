//! Hotkey 实现。
//! 工作方式：多个键时，最后一个键使用 [`TriggerType`] 定义的触发方式，其他键要保持按下。

use bevy::{
    ecs::schedule::BoxedCondition,
    prelude::*,
    utils::HashMap,
    window::PrimaryWindow,
};
use bevy_egui::EguiOutput;
use smallvec::SmallVec;

use crate::prelude::{ActionId, ActionRegistry};
use crate::utils::new_condition;
pub enum TriggerType {
    Pressed,
    Released,
    PressAndRelease,
    Repeat,
}

#[derive(Clone, Copy, Reflect, Debug)]
pub enum RuntimeTrigger {
    Pressed,
    Pressing,
    Released,
}

impl RuntimeTrigger {
    pub fn is_pressed(&self) -> bool {
        matches!(self, Self::Pressed)
    }
    pub fn is_pressing(&self) -> bool {
        matches!(self, Self::Pressing)
    }
    pub fn is_released(&self) -> bool {
        matches!(self, Self::Released)
    }
}

impl TriggerType {
    fn check_trigger(
        &self,
        code: KeyCode,
        input: &mut ButtonInput<KeyCode>,
    ) -> Option<RuntimeTrigger> {
        use TriggerType::*;
        let runtime_trigger = match self {
            Pressed if input.just_pressed(code) => Some(RuntimeTrigger::Pressed),
            Released if input.just_released(code) => Some(RuntimeTrigger::Released),
            PressAndRelease => input
                .just_pressed(code)
                .then_some(RuntimeTrigger::Pressed)
                .or_else(|| {
                    input
                        .just_released(code)
                        .then_some(RuntimeTrigger::Released)
                }),
            Repeat if input.pressed(code) => Some(RuntimeTrigger::Pressing),
            _ => None,
        };
        if input.just_released(code) {
            debug!("just released {code:?};");
        }
        input.clear_just_pressed(code);
        input.clear_just_released(code);
        runtime_trigger
    }
}

pub struct Hotkey {
    pub trigger_type: TriggerType,
    pub trigger_when: BoxedCondition,
    pub key: SmallVec<[KeyCode; 4]>,
}
const fn always() -> bool {
    true
}
impl Hotkey {
    pub fn new<M>(key: impl IntoIterator<Item = KeyCode>, trigger_when: impl Condition<M>) -> Self {
        Self::new_advanced(key, trigger_when, TriggerType::Pressed)
    }
    pub fn new_advanced<M>(
        key: impl IntoIterator<Item = KeyCode>,
        trigger_when: impl Condition<M>,
        trigger_type: TriggerType,
    ) -> Self {
        Self {
            trigger_type,
            trigger_when: new_condition(trigger_when),
            key: key.into_iter().collect(),
        }
    }
    pub fn new_global(key: impl IntoIterator<Item = KeyCode>) -> Self {
        Self::new(key, always)
    }
    /// 在应用于 `world` 前一定要先 `initialize`.
    pub fn initialize(&mut self, world: &mut World) {
        self.trigger_when.initialize(world);
    }

    pub fn keyboard_trigger(&self, world: &mut World) -> Option<RuntimeTrigger> {
        if self.key.is_empty() {
            return None;
        }
        let mut input = world.resource_mut::<ButtonInput<KeyCode>>();
        let mut other_all_pressed = true;
        for code in self.key.iter().take(self.key.len() - 1).copied() {
            other_all_pressed &= input.pressed(code);
        }
        other_all_pressed
            .then(|| {
                self.trigger_type
                    .check_trigger(*self.key.last().unwrap(), &mut input)
            })
            .flatten()
    }
    pub fn trigger_result(&mut self, world: &mut World) -> Option<RuntimeTrigger> {
        let not_editing_text = !world
            .query_filtered::<&EguiOutput, With<PrimaryWindow>>()
            .get_single(world)
            .map_or(false, |e| e.platform_output.mutable_text_under_cursor);
        let has_modifier = self.key.contains(&KeyCode::AltLeft)
            || self.key.contains(&KeyCode::AltRight)
            || self.key.contains(&KeyCode::ControlLeft)
            || self.key.contains(&KeyCode::ControlRight);

        (self.trigger_when.run_readonly((), world) && (not_editing_text || has_modifier))
            .then(|| self.keyboard_trigger(world))
            .flatten()
    }

    pub fn hotkey_text(&self) -> String {
        self.key
            .iter()
            .map(|k| format!("{k:?}"))
            .collect::<Vec<_>>()
            .join("+")
    }
}

#[derive(Resource, Default, Deref)]
pub struct HotkeyRegistry(HashMap<ActionId, SmallVec<[Hotkey; 3]>>);

pub struct HotkeyPlugin;

impl Plugin for HotkeyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HotkeyRegistry>();
        app.add_systems(
            PostUpdate,
            dispatch_hotkey.after(bevy_egui::EguiSet::ProcessOutput),
        );
    }
}

fn dispatch_hotkey(world: &mut World) {
    world.resource_scope(|world: &mut World, mut hotkeys: Mut<'_, HotkeyRegistry>| {
        for (id, listeners) in hotkeys.0.iter_mut() {
            for listener in listeners {
                if let Some(trigger) = listener.trigger_result(world) {
                    // todo: error handling
                    world
                        .resource_scope(
                            |world: &mut World, mut actions: Mut<'_, ActionRegistry>| {
                                actions
                                    .run_instant(id, trigger, world)
                                    .or_else(|_| actions.run_instant(id, (), world))
                            },
                        )
                        .expect("encountered err (todo handle this)");
                }
            }
        }
    });
}

pub trait HotkeysExt {
    fn register_hotkey(
        &mut self,
        id: impl Into<ActionId>,
        hotkeys: impl IntoIterator<Item = Hotkey>,
    ) -> &mut Self;
}

impl HotkeysExt for App {
    fn register_hotkey(
        &mut self,
        id: impl Into<ActionId>,
        hotkey_list: impl IntoIterator<Item = Hotkey>,
    ) -> &mut Self {
        self.world_mut().resource_scope(
            |world: &mut World, mut hotkeys: Mut<'_, HotkeyRegistry>| {
                let mut hotkey_list: SmallVec<[Hotkey; 3]> = hotkey_list
                    .into_iter()
                    .map(|mut k| {
                        k.initialize(world);
                        k
                    })
                    .collect();
                let listeners = hotkeys.0.entry(id.into()).or_default();
                listeners.append(&mut hotkey_list);
            },
        );
        self
    }
}
