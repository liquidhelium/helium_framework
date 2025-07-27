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
    items: &'a [MenuItem<C>],
    root: MenuNode,
}

enum MenuNode {
    Item(usize), // Index into items Vec
    SubMenu(String, IndexMap<String, MenuNode>),
}

impl<'a, C: 'static + Send + Sync> MenuTree<'a, C> {
    pub fn new(items: &'a [MenuItem<C>]) -> Self {
        Self::generate_tree(items)
    }

    fn generate_tree(items: &'a [MenuItem<C>]) -> Self {
        let mut root = MenuNode::SubMenu(String::new(), IndexMap::new());
        
        // Build a map from path to (index, item)
        let mut path_to_item: IndexMap<String, (usize, &'a MenuItem<C>)> = IndexMap::new();
        for (index, item) in items.iter().enumerate() {
            path_to_item.insert(item.path.clone(), (index, item));
        }
        
        // Build the tree recursively by checking Action type
        fn build_submenu_from_action<'a, C>(
            parent_path: &str,
            items: &'a [MenuItem<C>],
            path_to_item: &IndexMap<String, (usize, &'a MenuItem<C>)>,
        ) -> IndexMap<String, MenuNode> {
            let mut children = IndexMap::new();
            let mut items_by_parent: IndexMap<String, Vec<(usize, &'a MenuItem<C>)>> = IndexMap::new();
            
            // Group items by their parent path
            for (index, item) in items.iter().enumerate() {
                let item_parent = if let Some(last_slash) = item.path.rfind('/') {
                    &item.path[..last_slash]
                } else {
                    ""
                };
                
                if item_parent == parent_path {
                    items_by_parent.entry("".to_string()).or_default().push((index, item));
                }
            }
            
            // Sort by priority
            if let Some(items) = items_by_parent.get_mut("") {
                items.sort_by_key(|(_, item)| item.priority);
            }
            
            // Build children
            if let Some(items) = items_by_parent.get("") {
                for (index, item) in items {
                    let item_name = if let Some(last_slash) = item.path.rfind('/') {
                        &item.path[last_slash + 1..]
                    } else {
                        &item.path
                    };
                    
                    // Check if this item should be a submenu based on Action type
                    match item.action {
                        Action::SubMenu => {
                            // Build submenu by finding all items with this as prefix
                            let mut submenu_items = Vec::new();
                            let prefix = if parent_path.is_empty() {
                                item.path.clone()
                            } else {
                                format!("{}/ {}", parent_path, item_name)
                            };
                            
                            for (_, other_item) in items.iter() {
                                if other_item.path.starts_with(&prefix) && other_item.path != prefix {
                                    submenu_items.push(other_item);
                                }
                            }
                            
                            // Sort submenu items
                            submenu_items.sort_by_key(|item| item.priority);
                            
                            // Build submenu children
                            let mut submenu_children = IndexMap::new();
                            for sub_item in submenu_items {
                                let sub_name = &sub_item.path[prefix.len() + 1..];
                                if let Some((idx, _)) = path_to_item.get(&sub_item.path) {
                                    submenu_children.insert(sub_name.to_string(), MenuNode::Item(*idx));
                                }
                            }
                            
                            children.insert(item_name.to_string(), MenuNode::SubMenu(item.title.to_string(), submenu_children));
                        }
                        _ => {
                            children.insert(item_name.to_string(), MenuNode::Item(*index));
                        }
                    }
                }
            }
            
            children
        }
        
        // Build the root menu
        if let MenuNode::SubMenu(_, ref mut children) = root {
            *children = build_submenu_from_action("", items, &path_to_item);
        }
        
        Self { items, root }
    }

    pub fn render(&self, ui: &mut Ui, world: &mut World, context: &C) {
        self.render_recursive(&self.root, ui, world, context);
    }

    fn render_recursive(&self, node: &MenuNode, ui: &mut Ui, world: &mut World, context: &C) {
        match node {
            MenuNode::Item(item_index) => {
                let item = &self.items[*item_index];
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
                        unreachable!("SubMenu action should not be rendered directly");
                    }
                }
            }
            MenuNode::SubMenu(title, children) => {
                if !children.is_empty() {
                    ui.menu_button(title, |ui| {
                        for child in children.values() {
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