use std::any::{type_name, TypeId};
use std::intrinsics::transmute_unchecked;
use std::mem::transmute;
use std::sync::Arc;

use bevy::ecs::system::{RegisteredSystemError, SystemId, SystemParam};
use bevy::prelude::*;
use bevy::reflect::{TypeInfo, Typed};
use bevy::utils::HashMap;
use egui::mutex::Mutex;
use sealed::Sealed;
use snafu::{ResultExt, Snafu};

#[derive(Clone, Copy)]
pub struct ReflectSystemId {
    entity: Entity,
    in_type: TypeId,
    out_type: TypeId,
}

impl ReflectSystemId {
    pub fn from_system_id<I: bevy::prelude::SystemInput + 'static, O: 'static>(
        system_id: SystemId<I, O>,
    ) -> Self {
        Self::from_entity::<I,O>(system_id.entity())
    }
    pub fn system_id<I: bevy::prelude::SystemInput + 'static, O: 'static>(
        &self,
    ) -> Option<SystemId<I, O>> {
        if self.in_type == TypeId::of::<I>() && self.out_type == TypeId::of::<O>() {
            Some(SystemId::from_entity(self.entity))
        } else {
            None
        }
    }
    pub fn from_entity<I: bevy::prelude::SystemInput + 'static, O: 'static>(entity: Entity) -> Self {
        let in_type = TypeId::of::<I>();
        let out_type = TypeId::of::<O>();
        Self {
            entity,
            in_type,
            out_type,
        }
    }
}

fn run_system_reflect<'i, I, O>(
    world: &mut World,
    system_id: ReflectSystemId,
    input: I,
) -> Result<O, ActionError>
where
    I: SystemInput + InputSubset<'i> + 'static,
    I::Inner<'i>: Reflect,
    O: 'static + Reflect,
{
    let system_id: SystemId<I, O> = system_id.system_id().ok_or(ActionError::MismatchInput {
        // TODO
        expected_type_name: format!("{:?}", system_id.in_type),
        found_type_name: type_name::<I>().to_owned(),
    })?;
    let e = world.run_system_with_input(system_id, input.into_inner());
    if let Ok(output) = e {
        Ok(output)
    } else {
        Err(ActionError::RegistrationError {
            message: format!("Failed to run system with input"),
        })
    }
}

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
pub struct RSystemRegistry(HashMap<ActionId, ReflectSystemId>);

impl RSystemRegistry {
    pub fn run_instant<'i, I: InputSubset<'i>>(
        &mut self,
        id: &ActionId,
        input: I,
        world: &mut World,
    ) -> Result<(), ActionError>
    where
        <I as bevy::prelude::SystemInput>::Param<'static>: InputSubset<'static> + 'static,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: Reflect,
    {
        self.run_instant_ret::<I, ()>(id, input, world)
    }
    pub fn run_instant_ret<'i, I: InputSubset<'i>, O: Reflect>(
        &mut self,
        id: &ActionId,
        input: I,
        world: &mut World,
    ) -> Result<O, ActionError>
    where
        <I as bevy::prelude::SystemInput>::Param<'static>: InputSubset<'static> + 'static,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: Reflect,
    {
        self.0
            .get(id)
            .ok_or(ActionError::NotFound { id: id.to_string() })
            .map(|o| {
                run_system_reflect::<I::Param<'static>, O>(world, *o, unsafe {
                    transmute_unchecked::<_, I::Param<'static>>(input)
                })
            })?
    }
}

type BoxedFn = Box<dyn FnOnce(&mut World) + Send + Sync + 'static>;

pub trait DynActionStorage: Send + Sync {
    fn get_command(&self, input: Box<dyn Reflect>) -> Result<BoxedFn, String>;
    fn input_type_info(&self) -> &'static TypeInfo;
}

pub struct ActionStorage<Input: ActionArgument> {
    action: Arc<Mutex<Box<dyn System<In = In<Input>, Out = ()>>>>,
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
    storages: Res<'w, RSystemRegistry>,
}

impl Actions<'_, '_> {
    pub fn run_action<'i, I: InputSubset<'i> + Send + Sync>(
        &mut self,
        id: &ActionId,
        input: I,
    ) -> Result<(), ActionError>
    where
        <I as bevy::prelude::SystemInput>::Param<'static>:
            InputSubset<'static> + 'static + Send + Sync,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: Reflect,
    {
        if self.storages.0.contains_key(id) {
            let get = self.storages.0.get(id).copied();
            let input1 = unsafe { transmute_unchecked::<_, I::Param<'static>>(input) };
            let id1 = id.clone();
            self.commands.queue(move |world: &mut World| {
                if let Err(err)= get.map(|id| {
                    {
                        let system_id = id;
                        let system_id: SystemId<I::Param<'static>, ()> =
                            system_id.system_id().ok_or(ActionError::MismatchInput {
                                // TODO
                                expected_type_name: std::format!("{:?}", system_id.in_type),
                                found_type_name: type_name::<I::Param<'static>>().to_owned(),
                            })?;
                        let e = world.run_system_with_input(system_id, input1.into_inner());
                        if let Ok(output) = e {
                            Ok(output)
                        } else {
                            Err(ActionError::RegistrationError {
                                message: "Failed to run system with input".to_string(),
                            })
                        }
                    }
                })
                .unwrap() {
                    error!("Failed to run action {}: {:?}", id1, err);
                }
            });
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
    #[snafu(whatever)]
    RegistrationError { message: String },
}

pub trait ActionsExt {
    fn reflect_system<'i, M, I: InputSubset<'i> + Send + Sync +'static, O: Reflect>(
        &mut self,
        id: impl Into<ActionId>,
        description: impl Into<String>,
        action: impl IntoSystem<I, O, M> + 'static,
    ) -> &mut Self
    where
        <I as bevy::prelude::SystemInput>::Param<'static>:
            InputSubset<'static> + 'static + Send + Sync,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: Reflect;

}

mod sealed {
    use bevy::ecs::system::{In, InMut, InRef};

    pub trait Sealed {}
    impl<T> Sealed for In<T> {}
    impl Sealed for () {}
    impl<'i, T: 'static> Sealed for InRef<'i, T> {}
    impl<'i, T: 'static> Sealed for InMut<'i, T> {}
}

pub trait InputSubset<'i>: sealed::Sealed + SystemInput {
    fn into_inner(self) -> Self::Inner<'i>;
}
impl InputSubset<'static> for () {
    fn into_inner(self) -> Self::Inner<'static> {}
}
impl<T: 'static> InputSubset<'static> for In<T> {
    fn into_inner(self) -> Self::Inner<'static> {
        self.0
    }
}

impl<'a, T: 'static> InputSubset<'a> for InRef<'a, T> {
    fn into_inner(self) -> Self::Inner<'a> {
        self.0
    }
}
impl<'a, T: 'static> InputSubset<'a> for InMut<'a, T> {
    fn into_inner(self) -> Self::Inner<'a> {
        self.0
    }
}

impl ActionsExt for App {
    fn reflect_system<'i, M, I: InputSubset<'i> + Send + Sync +'static, O: Reflect>(
        &mut self,
        id: impl Into<ActionId>,
        description: impl Into<String>,
        action: impl IntoSystem<I, O, M> + 'static,
    ) -> &mut Self
    where
        <I as bevy::prelude::SystemInput>::Param<'static>:
            InputSubset<'static> + 'static + Send + Sync,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: Reflect,
    {
        let id = id.into();
        self.world_mut()
            .resource_scope(|world, mut actions: Mut<'_, RSystemRegistry>| {
                let rid = world.register_system(action);
                actions.0.insert(
                    id.clone(),
                    ReflectSystemId::from_entity::<I::Param<'static>, O>(
                        rid.entity(),
                    ),
                );
            });
        self
    }
}

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RSystemRegistry>();
    }
}
