use std::any::type_name;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::reflect::{TypeInfo, Typed};
use bevy::utils::HashMap;
use egui::mutex::Mutex;
use snafu::Snafu;

use crate::utils::identifier::Identifier;

pub struct BoxedStorage {
    boxed_action: Box<dyn DynActionStorage>,
    description: ActionDescription,
}

#[derive(Deref)]
pub struct ActionDescription {
    description: String,
}

impl BoxedStorage {
    fn get_command(&self, input: Box<dyn Reflect>) -> Result<BoxedFn, String> {
        self.boxed_action.get_command(input)
    }
    pub fn get_description(&self) -> &str {
        &self.description
    }
    pub fn input_type_info(&self) -> &'static TypeInfo {
        self.boxed_action.input_type_info()
    }
}

pub type ActionId = Identifier;

pub trait ActionArgument: Reflect + Typed {}

impl<T> ActionArgument for T where T: Reflect + Typed {}

#[derive(Resource, Default, Deref)]
pub struct ActionRegistry(HashMap<ActionId, BoxedStorage>);

impl ActionRegistry {
    pub fn run_instant<R: ActionArgument>(
        &mut self,
        id: &ActionId,
        input: R,
        world: &mut World,
    ) -> Result<(), ActionError> {
        self.0
            .get(id)
            .ok_or(ActionError::NotFound { id: id.to_string() })?
            .get_command(Box::new(input))
            .map_err(|expected| ActionError::MismatchInput {
                expected_type_name: expected,
                found_type_name: type_name::<R>().to_owned(),
            })?(world);
        Ok(())
    }
}

type BoxedFn = Box<dyn FnOnce(&mut World) + Send + Sync + 'static>;

pub trait DynActionStorage: Send + Sync {
    fn get_command(&self, input: Box<dyn Reflect>) -> Result<BoxedFn, String>;
    fn input_type_info(&self) -> &'static TypeInfo;
}

pub struct ActionStorage<Input: ActionArgument> {
    action: Arc<Mutex<Box<dyn System<In = Input, Out = ()>>>>,
}

impl<Input: ActionArgument> DynActionStorage for ActionStorage<Input> {
    fn get_command(
        &self,
        input: Box<dyn Reflect>,
    ) -> Result<Box<dyn FnOnce(&mut World) + Send + Sync + 'static>, String> {
        let owned_action = Arc::clone(&self.action);
        let input = *input
            .into_any()
            .downcast::<Input>()
            .map_err(|_| type_name::<Input>().to_string())?;
        Ok(Box::new(move |world| {
            let lock = &mut owned_action.lock();
            lock.run(input, world);
            lock.apply_deferred(world);
        }))
    }
    fn input_type_info(&self) -> &'static TypeInfo {
        Input::type_info()
    }
}

#[derive(SystemParam)]
pub struct Actions<'w, 's> {
    commands: Commands<'w, 's>,
    storages: Res<'w, ActionRegistry>,
}

impl Actions<'_, '_> {
    pub fn run_action<I: ActionArgument>(
        &mut self,
        id: &ActionId,
        input: I,
    ) -> Result<(), ActionError> {
        if self.storages.0.contains_key(id) {
            self.commands.add(
                self.storages
                    .0
                    .get(id)
                    .unwrap()
                    .get_command(Box::new(input))
                    .map_err(|expected_type_name| ActionError::MismatchInput {
                        expected_type_name,
                        found_type_name: type_name::<I>().into(),
                    })?,
            );
            Ok(())
        } else {
            Err(ActionError::NotFound { id: id.to_string() })
        }
    }
}

#[derive(Snafu, Debug)]
pub enum ActionError {
    #[snafu(display("Action {id} does not exist."))]
    NotFound { id: String },
    #[snafu(display(
        "input type mismatch, expecting {expected_type_name}, found {found_type_name}"
    ))]
    MismatchInput {
        expected_type_name: String,
        found_type_name: String,
    },
}

pub trait ActionsExt {
    fn register_action<M, In: ActionArgument>(
        &mut self,
        id: impl Into<ActionId>,
        description: impl Into<String>,
        action: impl IntoSystem<In, (), M>,
    ) -> &mut Self;
}

impl ActionsExt for App {
    fn register_action<M, SystemInput: ActionArgument>(
        &mut self,
        id: impl Into<ActionId>,
        description: impl Into<String>,
        action: impl IntoSystem<SystemInput, (), M>,
    ) -> &mut Self {
        self.world_mut()
            .resource_scope(|world, mut actions: Mut<'_, ActionRegistry>| {
                let mut system = IntoSystem::into_system(action);
                system.initialize(world);
                actions.0.insert(
                    id.into(),
                    BoxedStorage {
                        boxed_action: Box::new(ActionStorage {
                            action: Arc::new(Mutex::new(Box::new(system))),
                        }),
                        description: ActionDescription {
                            description: description.into(),
                        },
                    },
                );
            });
        self
    }
}

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionRegistry>();
    }
}
