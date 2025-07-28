use std::any::{type_name, TypeId};
use std::intrinsics::transmute_unchecked;

use bevy::ecs::system::{SystemId, SystemParam};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::Typed;
use snafu::Snafu;

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
        Self::from_entity::<I, O>(system_id.entity())
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
    pub fn from_entity<I: bevy::prelude::SystemInput + 'static, O: 'static>(
        entity: Entity,
    ) -> Self {
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
    meta: &ReflectSystemMeta,
    input: I,
) -> Result<O, ActionError>
where
    I: SystemInput + InputSubset<'i> + 'static,
    I::Inner<'i>: 'static,
    O: 'static,
{
    let system_id = meta.system_id;
    let system_id: SystemId<I, O> = system_id.system_id().ok_or(ActionError::MismatchInput {
        // TODO
        expected_type_name: meta.input.clone(),
        found_type_name: type_name::<I>().to_owned(),
    })?;
    let e = world.run_system_with(system_id, input.into_inner());
    if let Ok(output) = e {
        Ok(output)
    } else {
        Err(ActionError::RegistrationError {
            message: "Failed to run system with input".to_string(),
        })
    }
}

use crate::utils::identifier::Identifier;

#[derive(Deref)]
pub struct ActionDescription {
    description: String,
}

pub type ActionId = Identifier;

pub trait ActionArgument: 'static + Typed {}

impl<T> ActionArgument for T where T: 'static + Typed {}

#[derive(Clone)]
pub struct ReflectSystemMeta {
    pub id: ActionId,
    pub description: String,
    pub system_id: ReflectSystemId,
    pub input: String,
    pub output: String,
}

#[derive(Resource, Default, Deref)]
pub struct RSystemRegistry(HashMap<ActionId, ReflectSystemMeta>);

impl RSystemRegistry {
    pub fn run_instant<'i, I: InputSubset<'i>>(
        &mut self,
        id: &ActionId,
        input: I,
        world: &mut World,
    ) -> Result<(), ActionError>
    where
        <I as bevy::prelude::SystemInput>::Param<'static>: InputSubset<'static> + 'static,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: 'static,
    {
        self.run_instant_ret::<I, ()>(id, input, world)
    }
    pub fn run_instant_ret<'i, I: InputSubset<'i>, O: 'static>(
        &mut self,
        id: &ActionId,
        input: I,
        world: &mut World,
    ) -> Result<O, ActionError>
    where
        <I as bevy::prelude::SystemInput>::Param<'static>: InputSubset<'static> + 'static,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: 'static,
    {
        self.0
            .get(id)
            .ok_or(ActionError::NotFound { id: id.to_string() })
            .map(|meta| {
                run_system_reflect::<I::Param<'static>, O>(world, meta, unsafe {
                    transmute_unchecked::<_, I::Param<'static>>(input)
                })
            })?
    }

    pub fn get_meta(&self, id: &ActionId) -> Option<&ReflectSystemMeta> {
        self.0.get(id)
    }
}

#[derive(SystemParam)]
pub struct Actions<'w, 's> {
    commands: Commands<'w, 's>,
    storages: Res<'w, RSystemRegistry>,
}

impl Actions<'_, '_> {
    pub fn run_action<'i, I: InputSubset<'i> + Send + Sync + 'static>(
        &mut self,
        id: &ActionId,
        input: I,
    ) -> Result<(), ActionError>
    where
        <I as bevy::prelude::SystemInput>::Param<'static>:
            InputSubset<'static> + 'static + Send + Sync,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: 'static,
    {
        if self.storages.0.contains_key(id) {
            let get = self.storages.0.get(id).cloned();
            let input1 = unsafe { transmute_unchecked::<_, I::Param<'static>>(input) };
            let id1 = id.clone();
            self.commands.queue(move |world: &mut World| {
                if let Err(err) = get
                    .map(|meta| {
                        {
                            let system_id = meta.system_id;
                            let system_id: SystemId<I::Param<'static>, ()> =
                                system_id.system_id().ok_or(ActionError::MismatchInput {
                                    // TODO
                                    expected_type_name: meta.input,
                                    found_type_name: type_name::<I::Param<'static>>().to_owned(),
                                })?;
                            let e = world.run_system_with(system_id, input1.into_inner());
                            if let Ok(output) = e {
                                Ok(output)
                            } else {
                                Err(ActionError::RegistrationError {
                                    message: "Failed to run system with input".to_string(),
                                })
                            }
                        }
                    })
                    .unwrap()
                {
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
    fn reflect_system<'i, M, I: InputSubset<'i> + Send + Sync + 'static, O: 'static>(
        &mut self,
        id: impl Into<ActionId>,
        description: impl Into<String>,
        action: impl IntoSystem<I, O, M> + 'static,
    ) -> &mut Self
    where
        <I as bevy::prelude::SystemInput>::Param<'static>:
            InputSubset<'static> + 'static + Send + Sync,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: 'static;
}

mod sealed {
    use bevy::ecs::system::{In, InMut, InRef};

    pub trait Sealed {}
    impl<T> Sealed for In<T> {}
    impl<'i, T: 'static> Sealed for InRef<'i, T> {}
    impl<'i, T: 'static> Sealed for InMut<'i, T> {}
    macro_rules! impl_sealed_tuple {
        ($($name:ident),*) => {
            impl<$($name: Sealed),*> Sealed for ($($name,)*) {}
        };
    }
    variadics_please::all_tuples!(
        impl_sealed_tuple,
        0, 8, I
    );
}

pub trait InputSubset<'i>: sealed::Sealed + SystemInput {
    fn into_inner(self) -> Self::Inner<'i>;
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

macro_rules! impl_system_input_tuple {
    ($(($n:tt, $name:ident)),*) => {
        #[allow(clippy::unused_unit)]
        impl<'i, $($name: InputSubset<'i>),*> InputSubset<'i> for ($($name,)*) {
            fn into_inner(self) -> Self::Inner<'i> {
                ($(
                    self.$n.into_inner(),
                )*)
            }
        }
    };
}

variadics_please::all_tuples_enumerated!(
    impl_system_input_tuple,
    0, 8, I
);

impl ActionsExt for App {
    fn reflect_system<'i, M, I: InputSubset<'i> + Send + Sync + 'static, O: 'static>(
        &mut self,
        id: impl Into<ActionId>,
        description: impl Into<String>,
        action: impl IntoSystem<I, O, M> + 'static,
    ) -> &mut Self
    where
        <I as bevy::prelude::SystemInput>::Param<'static>:
            InputSubset<'static> + 'static + Send + Sync,
        <<I as bevy::prelude::SystemInput>::Param<'static> as SystemInput>::Inner<'static>: 'static,
    {
        let id = id.into();
        let description = description.into();
        let input_type_name = type_name::<I::Param<'static>>().to_string();
        let output_type_name = type_name::<O>().to_string();
        self.world_mut()
            .resource_scope(|world, mut actions: Mut<'_, RSystemRegistry>| {
                let rid = world.register_system(action);
                let meta = ReflectSystemMeta {
                    id: id.clone(),
                    description: description.clone(),
                    system_id: ReflectSystemId::from_entity::<I::Param<'static>, O>(rid.entity()),
                    input: input_type_name,
                    output: output_type_name,
                };
                actions.0.insert(id.clone(), meta);
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn test_reflect_system() {
        let mut app = App::new();
        app
            .add_plugins(ActionPlugin);
        app.reflect_system(
            "test_action",
            "This is a test action",
            |In(input): In<i32>| {
                info!("Running test action with input: {}", input);
                input * 2
            },
        );
        app.reflect_system(
            "test_action_multi",
            "This is a test action",
            |(In(input), InRef(input1)): (In<i32>, InRef<i32>)| {
                info!("Running test action with input: {}", input);
                input + input1 + 10
            },
        );
        app.world_mut().resource_scope(
            |world: &mut World, mut actions: Mut<'_, RSystemRegistry>| {
                let result = actions.run_instant_ret::<In<i32>, i32>(
                    &"test_action".into(),
                    In(5),
                    world,
                );
                assert_eq!(result.unwrap(), 10);
                let result = actions.run_instant_ret::<(In<i32>, InRef<i32>), i32>(
                    &"test_action_multi".into(),
                    (In(5), InRef(&10)),
                    world,
                );
                assert_eq!(result.unwrap(), 25);
                // assert_eq!(input2, 1145);
            },
        );
    }
}