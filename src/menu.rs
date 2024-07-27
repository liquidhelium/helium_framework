use std::{borrow::Cow, fmt::Debug};

use bevy::{
    app::{App, Plugin}, ecs::schedule::{BoxedCondition, Condition}, prelude::{Deref, DerefMut, Mut, Resource, World}
};
use egui::Ui;
use enum_dispatch::enum_dispatch;
use indexmap::IndexMap;
use snafu::Snafu;

use crate::{
    prelude::{ActionId, ActionRegistry},
    utils::new_condition,
};

pub fn show_menu_ui(ui: &mut Ui, world: &mut World) {
    world.resource_scope(|world: &mut World, mut entry:Mut<EditorMenuEntrys>| {
        entry.0.foreach_ui(ui, world);
    });
}

#[enum_dispatch(MenuItemProvider)]
#[derive(Debug)]
pub enum MenuItemVariant {
    Button,
    Custom,
    SubMenu,
    Category,
}

#[derive(Debug)]
pub struct MenuItem {
    pub name: Cow<'static, str>,
    pub source: MenuItemVariant,
    pub piority: usize,
}

#[enum_dispatch]
pub trait MenuItemProvider {
    fn ui(&mut self, ui: &mut Ui, world: &mut World, name: &str);
    fn initialize(&mut self, _world: &mut World) {}
    // todo: move these methods into ItemAsContainer
    fn find_subitem_mut(&mut self, _id: &str) -> Option<&mut MenuItem> {
        None
    }
    fn find_subitem_recursive(&mut self, id: &str) -> Option<&mut MenuItem> {
        let split: Vec<&str> = id.splitn(2, '.').collect();
        let id = *split.first()?;
        let item = self.find_subitem_mut(id)?;
        if let Some(trailing) = split.get(1) {
            item.source.find_subitem_recursive(trailing)
        } else {
            Some(item)
        }
    }
    fn as_container(&mut self) -> Option<ItemAsContainer> {
        None
    }
}

pub trait ContainerItem<'item> {
    fn add_item(&mut self, id: &str, menu_item: MenuItem);
    fn remove_item(&mut self, id: &str) -> Option<MenuItem>;
    fn get_item(&self, id: &str) -> Option<&MenuItem>;
    fn get_item_mut(&mut self, id: &str) -> Option<&mut MenuItem>;
}

pub struct ItemAsContainer<'item> {
    container_item: Box<dyn ContainerItem<'item> + 'item>,
}
impl ItemAsContainer<'_> {
    pub fn add_item(&mut self, id: &str, menu_item: MenuItem) {
        self.container_item.add_item(id, menu_item)
    }
    pub fn remove_item(&mut self, id: &str) -> Option<MenuItem> {
        self.container_item.remove_item(id)
    }
    pub fn get_item(&self, id: &str) -> Option<&MenuItem> {
        self.container_item.get_item(id)
    }
    pub fn get_item_mut(&mut self, id: &str) -> Option<&mut MenuItem> {
        self.container_item.get_item_mut(id)
    }
}

pub struct Button {
    action: ActionId,
    avalible: BoxedCondition,
}

impl Button {
    pub fn new(action: impl Into<ActionId>) -> Self {
        Self {
            action: action.into(),
            avalible: new_condition(|| true),
        }
    }
    pub fn new_conditioned<M>(action: impl Into<ActionId>, available: impl Condition<M>) -> Self {
        Self {
            action: action.into(),
            avalible: new_condition(available),
        }
    }
}

impl Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Button>")
    }
}

impl MenuItemProvider for Button {
    fn ui(&mut self, ui: &mut Ui, world: &mut World, name: &str) {
        ui.add_enabled_ui(self.avalible.run_readonly((), world), |ui| {
            if ui.button(name).clicked() {
                world.resource_scope(|world: &mut World, mut actions: Mut<ActionRegistry>| {
                    let _ = actions.run_instant(&self.action, (), world).map_err(|err| {
                        bevy::prelude::error!("encountered error when running action: {}", err)
                    });
                });
                ui.close_menu();
            }
        });
    }
    fn initialize(&mut self, world: &mut World) {
        self.avalible.initialize(world);
    }
}

pub trait UiFunc: Fn(&mut Ui, &mut World, &str) + Sync + Send + 'static {}

impl<T> UiFunc for T where T: Fn(&mut Ui, &mut World, &str) + Sync + Send + 'static {}

pub struct Custom(pub Box<dyn UiFunc>);

impl MenuItemProvider for Custom {
    fn ui(&mut self, ui: &mut Ui, world: &mut World, name: &str) {
        (self.0)(ui, world, name)
    }
}

impl Debug for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<Custom widget>")
    }
}

#[derive(Debug, Default)]
pub struct ItemGroup {
    items: IndexMap<String, MenuItem>,
}

impl ItemGroup {
    pub fn iter_items_mut(&mut self) -> impl Iterator<Item = &mut MenuItem> {
        self.items.values_mut()
    }
    pub fn foreach_ui(&mut self, ui: &mut Ui, world: &mut World) {
        for item in self.iter_items_mut() {
            item.source.ui(ui, world, &item.name);
        }
    }
    pub fn as_container(&mut self) -> ItemAsContainer {
        ItemAsContainer {
            container_item: Box::new(ItemGroupAsContainer { group: self }),
        }
    }
}

struct ItemGroupAsContainer<'group> {
    group: &'group mut ItemGroup,
}

impl<'item> ContainerItem<'item> for ItemGroupAsContainer<'item> {
    fn add_item(&mut self, id: &str, menu_item: MenuItem) {
        self.group.items.insert(id.to_string(), menu_item);
        self.group
            .items
            .sort_unstable_by(|_, item1, _, item2| item1.piority.cmp(&item2.piority));
    }
    fn get_item_mut(&mut self, id: &str) -> Option<&mut MenuItem> {
        self.group.items.get_mut(id)
    }
    fn get_item(&self, id: &str) -> Option<&MenuItem> {
        self.group.items.get(id)
    }
    fn remove_item(&mut self, id: &str) -> Option<MenuItem> {
        self.group.items.shift_remove(id)
    }
}

#[derive(Debug, Default)]
pub struct SubMenu {
    group: ItemGroup,
}

#[derive(Debug, Default)]
pub struct Category {
    group: ItemGroup,
}
impl MenuItemProvider for SubMenu {
    fn ui(&mut self, ui: &mut Ui, world: &mut World, name: &str) {
        ui.menu_button(name, |ui| self.group.foreach_ui(ui, world));
    }
    fn find_subitem_mut(&mut self, sub_id: &str) -> Option<&mut MenuItem> {
        self.group.items.get_mut(sub_id)
    }
    fn as_container(&mut self) -> Option<ItemAsContainer<'_>> {
        Some(self.group.as_container())
    }
}

impl MenuItemProvider for Category {
    fn ui(&mut self, ui: &mut Ui, world: &mut World, name: &str) {
        ui.label(name);
        ui.separator();
        self.group.foreach_ui(ui, world);
    }
    fn find_subitem_mut(&mut self, sub_id: &str) -> Option<&mut MenuItem> {
        self.group.items.get_mut(sub_id)
    }
    fn as_container(&mut self) -> Option<ItemAsContainer> {
        Some(self.group.as_container())
    }
}
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorMenuEntrys>();
    }
}

#[derive(DerefMut, Deref, Resource, Default)]
pub struct EditorMenuEntrys(ItemGroup);

pub trait MenuExt {
    fn menu_context(&mut self, add_menu: impl FnOnce(MenuContext)) -> &mut Self;
}

pub struct MenuContext<'w> {
    item: ItemAsContainer<'w>,
    world: &'w mut World,
}

impl MenuContext<'_> {
    pub fn inside_sub<'a>(
        &mut self,
        id: &'a str,
        add_sub: impl FnOnce(MenuContext),
    ) -> Result<(), MenuError<'a>> {
        let item = self
            .item
            .get_item_mut(id)
            .ok_or(MenuError::NotFound { id })?;
        add_sub(MenuContext {
            item: item
                .source
                .as_container()
                .ok_or(MenuError::NotAContainer { id })?,
            world: self.world,
        });
        Ok(())
    }
    pub fn with_category(
        &mut self,
        id: &str,
        name: Cow<'static, str>,
        piority: usize,
        add_sub: impl FnOnce(MenuContext),
    ) {
        self.add(id, name, Category::default(), piority);
        self.inside_sub(id, add_sub).unwrap();
    }
    pub fn with_sub_menu(
        &mut self,
        id: &str,
        name: Cow<'static, str>,
        piority: usize,
        add_sub: impl FnOnce(MenuContext),
    ) {
        self.add(id, name, SubMenu::default(), piority);
        self.inside_sub(id, add_sub).unwrap();
    }
    pub fn add(
        &mut self,
        id: &str,
        name: Cow<'static, str>,
        item: impl Into<MenuItemVariant>,
        piority: usize,
    ) {
        let mut source = item.into();
        source.initialize(self.world);
        self.item.add_item(
            id,
            MenuItem {
                name,
                source,
                piority,
            },
        );
    }
}

impl MenuExt for App {
    fn menu_context(&mut self, add_menu: impl FnOnce(MenuContext)) -> &mut Self {
        self.world_mut().resource_scope(
            |world, mut entrys: bevy::prelude::Mut<'_, EditorMenuEntrys>| {
                let container = entrys.as_container();
                add_menu(MenuContext {
                    item: container,
                    world,
                });
            },
        );
        self
    }
}

#[derive(Debug, Snafu)]
pub enum MenuError<'a> {
    #[snafu(display("Id {id} not found"))]
    NotFound { id: &'a str },
    #[snafu(display("{id} is not a container"))]
    NotAContainer { id: &'a str },
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_item() {
        let mut menu = dbg!(construct_menu());
        dbg!(menu.source.find_subitem_recursive("item1"));
        dbg!(menu.source.find_subitem_recursive("category2.item1"));
        dbg!(menu
            .source
            .find_subitem_recursive("category2.item1.nonexist"));
    }

    fn construct_menu() -> MenuItem {
        let category = MenuItem {
            name: "category".into(),
            source: Category {
                group: ItemGroup {
                    items: [
                        ("item1".into(), button_with_name("Item1".into())),
                        ("item2".into(), button_with_name("Item2".into())),
                    ]
                    .into_iter()
                    .collect(),
                },
            }
            .into(),
            piority: 0,
        };

        MenuItem {
            name: "menu".into(),
            source: SubMenu {
                group: ItemGroup {
                    items: [
                        ("item1".into(), button_with_name("Item1".into())),
                        ("category2".into(), category),
                    ]
                    .into_iter()
                    .collect(),
                },
            }
            .into(),
            piority: 1,
        }
    }

    fn button_with_name(name: Cow<'static, str>) -> MenuItem {
        MenuItem {
            name,
            source: Button::new("wtf.is.this").into(),
            piority: 0,
        }
    }
}
