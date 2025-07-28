use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::prelude::*;
use egui::Ui;
use indexmap::IndexMap;

use crate::reflect_system::ActionId;

// Core menu item generic over context type
pub struct MenuItem<C> {
    pub id: String,
    pub title: Cow<'static, str>,
    pub path: String,
    pub action: Action<C>,
    pub when: Option<Box<dyn Fn(&World, &C) -> bool + Send + Sync>>,
    pub priority: i32,
}

impl<C> MenuItem<C> {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        path: impl Into<String>,
        action: Action<C>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            path: path.into(),
            action,
            when: None,
            priority: 0,
        }
    }

    pub fn with_condition(
        mut self,
        condition: impl Fn(&World, &C) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.when = Some(Box::new(condition));
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

// Action enum generic over context
pub enum Action<C> {
    Command(ActionId, PhantomData<C>),
    Custom(Box<dyn Fn(&mut Ui, &mut World, &C) + Send + Sync>),
    SubMenu,
}

// Type-safe storage using type as key
#[derive(Resource, Default)]
pub struct MenuSystem {
    menus: HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl MenuSystem {
    // Type-safe insertion
    pub fn register<C: 'static + Send + Sync>(&mut self, item: MenuItem<C>) {
        let items: &mut Vec<MenuItem<C>> = self.menus
            .entry(std::any::TypeId::of::<C>())
            .or_insert_with(|| Box::new(Vec::<MenuItem<C>>::new()))
            .downcast_mut::<Vec<MenuItem<C>>>()
            .unwrap();
        items.push(item);
        items.sort_by_key(|item| item.priority);
    }

    // Type-safe retrieval
    pub fn get_items<C: 'static + Send + Sync>(&self) -> &[MenuItem<C>] {
        self.menus
            .get(&std::any::TypeId::of::<C>())
            .and_then(|items| items.downcast_ref::<Vec<MenuItem<C>>>())
            .map(|vec| vec.as_slice())
            .unwrap_or_default()
    }

    // Type-safe rendering
    pub fn show_menu<C: 'static + Send + Sync>(&self, ui: &mut Ui, world: &mut World, context: &C) {
        let items = self.get_items::<C>();
        let tree = MenuTree::<C>::new(items);
        
        tree.render(ui, world, context);
    }
}

// MenuTree for hierarchical rendering
pub struct MenuTree<'a, C> {
    root: MenuNode<'a, C>,
}

enum MenuNode<'a, C> {
    Item(&'a MenuItem<C>),
    SubMenu(String, IndexMap<String, MenuNode<'a, C>>),
}

impl<'a, C: 'static + Send + Sync> MenuTree<'a, C> {
    pub fn new(items: &'a [MenuItem<C>]) -> Self {
        Self::generate_tree(items)
    }

fn generate_tree(items: &'a [MenuItem<C>]) -> Self {
        let mut root = MenuNode::SubMenu(String::new(), IndexMap::new());

        // Helper function to build the entire tree structure
        fn build_recursive<'a, C>(
            items: &'a [MenuItem<C>],
            parent_path: &str,
            used_items: &mut std::collections::HashSet<usize>,
        ) -> IndexMap<String, MenuNode<'a, C>> {
            let mut children = IndexMap::new();
            let mut direct_children = Vec::new();

            // Find all direct children of this path
            for (index, item) in items.iter().enumerate() {
                if used_items.contains(&index) {
                    continue;
                }

                let item_parent = if let Some(last_slash) = item.path.rfind('/') {
                    &item.path[..last_slash]
                } else {
                    ""
                };

                if item_parent == parent_path {
                    direct_children.push((index, item));
                }
            }

            // Sort by priority
            direct_children.sort_by_key(|(_, item)| item.priority);

            // Build the tree structure
            for (index, item) in direct_children {
                let item_name = if let Some(last_slash) = item.path.rfind('/') {
                    &item.path[last_slash + 1..]
                } else {
                    &item.path
                };

                match item.action {
                    Action::SubMenu => {
                        // This is a submenu - recursively build its children
                        let submenu_path = &item.path;
                        let submenu_children = build_recursive(items, submenu_path, used_items);
                        children.insert(item_name.to_string(), MenuNode::SubMenu(item.title.to_string(), submenu_children));
                    }
                    _ => {
                        // This is a regular menu item
                        children.insert(item_name.to_string(), MenuNode::Item(item));
                    }
                }
            }

            children
        }

        // Build the complete tree structure
        let mut used_items = std::collections::HashSet::new();
        if let MenuNode::SubMenu(_, ref mut children) = root {
            *children = build_recursive(items, "", &mut used_items);
        }

        Self { root }
    }

    pub fn render(&self, ui: &mut Ui, world: &mut World, context: &C) {
        self.render_recursive(&self.root, ui, world, context);
    }

    fn render_recursive(&self, node: &MenuNode<'a, C>, ui: &mut Ui, world: &mut World, context: &C) {
        match node {
            MenuNode::Item(item) => {
                let visible = if let Some(ref condition) = item.when {
                    condition(world, context)
                } else {
                    true
                };

                if !visible {
                    return;
                }

                match &item.action {
                    Action::Command(action_id, _) => {
                        if ui.button(&*item.title).clicked() {
                            world.resource_scope(|world, mut actions: Mut<crate::reflect_system::RSystemRegistry>| {
                                let _ = actions.run_instant(action_id, (), world);
                            });
                        }
                    }
                    Action::Custom(render_fn) => {
                        render_fn(ui, world, context);
                    }
                    Action::SubMenu => {
                        // SubMenu items are handled by the tree structure
                    }
                }
            }
            MenuNode::SubMenu(title, children) => {
                if !children.is_empty() {
                    ui.menu_button(title, |ui| {
                        // Sort children by priority
                        let mut sorted_children: Vec<_> = children.iter().collect();
                        sorted_children.sort_by_key(|(_, node)| match *node {
                            MenuNode::Item(item) => item.priority,
                            MenuNode::SubMenu(_, _) => 0,
                        });
                        
                        for (_, child) in sorted_children {
                            self.render_recursive(child, ui, world, context);
                        }
                    });
                }
            }
        }
    }
}

// Menu registration trait
pub trait MenuRegistration {
    fn register_submenu<C>(&mut self, path: impl Into<String>, id: impl Into<String>, title: impl Into<Cow<'static, str>>) -> &mut Self
where
    C: 'static + Send + Sync,;
    fn register<C: 'static + Send + Sync>(&mut self, item: MenuItem<C>) -> &mut Self;
    
    fn register_command<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        id: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        command: impl Into<ActionId>,
    ) -> &mut Self;
    
    fn register_custom<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        id: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        render: impl Fn(&mut Ui, &mut World, &C) + Send + Sync + 'static,
    ) -> &mut Self;
}

impl MenuRegistration for App {
    fn register<C: 'static + Send + Sync>(&mut self, item: MenuItem<C>) -> &mut Self {
        self.world_mut().resource_scope(|world, mut menu_system: Mut<MenuSystem>| {
            menu_system.register(item);
        });
        self
    }
    
    fn register_command<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        id: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        command: impl Into<ActionId>,
    ) -> &mut Self {
        self.register(MenuItem::new(
            id,
            title,
            path,
            Action::Command(command.into(), PhantomData::<C>),
        ))
    }
    
    fn register_custom<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        id: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        render: impl Fn(&mut Ui, &mut World, &C) + Send + Sync + 'static,
    ) -> &mut Self {
        self.register(MenuItem::new(
            id,
            title,
            path,
            Action::Custom(Box::new(render)),
        ))
    }

    fn register_submenu<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        id: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        self.register(MenuItem::<C>::new(
            id,
            title,
            path,
            Action::SubMenu,
        ))
    } 
}

// Plugin for the new menu system
pub struct MenuSystemPlugin;

impl Plugin for MenuSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuSystem>();
    }
}