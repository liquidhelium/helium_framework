use std::borrow::Cow;
use std::collections::HashMap;
use std::marker::PhantomData;

use bevy::ecs::schedule::BoxedCondition;
use bevy::prelude::*;
use egui::Ui;
use indexmap::IndexMap;

use crate::reflect_system::{ActionId, RSystemRegistry, ReflectSystemId};
use crate::utils::new_condition;

/// 菜单项结构体，用于定义菜单中的单个项目
///
/// 这是一个泛型结构体，可以适应不同的上下文类型
///
/// # 示例
/// ```
/// use helium_framework::menu_system::MenuItem;
/// use std::marker::PhantomData;
/// use crate::helium_framework::menu_system::Action;
///
/// let item = MenuItem::new("打开文件", "文件/打开", Action::Command("open_file".into(), PhantomData::<()>));
/// ```
pub struct MenuItem<C> {
    /// 菜单项的显示文本
    pub title: Cow<'static, str>,
    /// 菜单项的路径，用于构建层级结构（例如："文件/打开"）
    pub path: String,
    /// 菜单项的动作类型
    pub action: Action<C>,
    /// 可选的显示条件，返回true时显示此菜单项
    pub when: BoxedCondition,
    /// 优先级，值越小显示越靠前
    pub priority: i32,
}

impl<C> MenuItem<C> {
    /// 创建一个新的菜单项
    ///
    /// # 参数
    /// - `title`: 菜单项的显示文本
    /// - `path`: 菜单项的路径，用于构建层级结构
    /// - `action`: 菜单项的动作类型
    ///
    /// # 示例
    /// ```
    /// let item = MenuItem::new("保存", "文件/保存", Action::Command("save_file".into(), PhantomData::<()>));
    /// ```
    pub fn new(
        title: impl Into<Cow<'static, str>>,
        path: impl Into<String>,
        action: Action<C>,
    ) -> Self {
        Self {
            title: title.into(),
            path: path.into(),
            action,
            when: new_condition(|| true),
            priority: 0,
        }
    }

    /// 为菜单项添加显示条件
    ///
    /// # 参数
    /// - `condition`: 条件函数，当返回true时显示此菜单项
    ///
    /// # 示例
    /// ```
    /// let item = MenuItem::new("save", "保存", "文件/保存", Action::Command("save_file".into(), PhantomData::<()>))
    ///     .with_condition(|world: &World| world.resource::<AppState>().is_document_opened);
    /// ```
    pub fn with_condition<M>(mut self, condition: impl Condition<M>) -> Self {
        self.when = new_condition(condition);
        self
    }

    /// 设置菜单项的优先级
    ///
    /// # 参数
    /// - `priority`: 优先级数值，值越小显示越靠前
    ///
    /// # 示例
    /// ```
    /// let item = MenuItem::new("new", "新建", "文件/新建", Action::Command("new_file".into(), PhantomData::<()>))
    ///     .with_priority(0); // 高优先级
    /// ```
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// 初始化菜单项的条件系统
    ///
    /// # 参数
    /// - `world`: Bevy的World引用
    pub fn initialize(&mut self, world: &mut World) {
        self.when.initialize(world);
    }
}

/// 菜单项动作枚举，定义菜单项的行为类型
///
/// 这是一个泛型枚举，支持三种不同类型的动作：
/// - 命令执行：执行一个已注册的反射系统
/// - 自定义动作：执行一个自定义系统
/// - 子菜单：包含子菜单项
pub enum Action<C> {
    /// 执行一个已注册的命令（反射系统）
    ///
    /// 第一个参数是命令的ActionId，第二个参数是上下文类型的占位符
    Command(ActionId, PhantomData<C>),

    /// 执行一个自定义系统
    ///
    /// 参数是自定义系统的标识符
    Custom(ActionId),

    /// 表示这是一个子菜单项，会包含子菜单
    SubMenu,
}

/// 菜单系统，用于管理所有菜单项
///
/// 使用类型安全的存储方式，根据上下文类型进行分组管理
/// 支持不同类型的上下文同时存在，互不干扰
#[derive(Resource, Default)]
pub struct MenuSystem {
    menus: HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl MenuSystem {
    /// 注册一个新的菜单项到系统中
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `item`: 要注册的菜单项
    /// - `world`: Bevy的World引用，用于初始化条件系统
    ///
    /// # 说明
    /// 菜单项会根据上下文类型自动分组，相同类型的菜单项会被放在一起
    /// 注册后会自动按优先级排序
    pub fn register<C: 'static + Send + Sync>(&mut self, mut item: MenuItem<C>, world: &mut World) {
        let items: &mut Vec<MenuItem<C>> = self
            .menus
            .entry(std::any::TypeId::of::<C>())
            .or_insert_with(|| Box::new(Vec::<MenuItem<C>>::new()))
            .downcast_mut::<Vec<MenuItem<C>>>()
            .unwrap();

        // 初始化条件系统
        item.when.initialize(world);
        items.push(item);
        items.sort_by_key(|item| item.priority);
    }

    /// 获取指定上下文类型的所有菜单项的只读引用
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 返回值
    /// 返回指定类型的菜单项切片，如果没有则返回空切片
    pub fn get_items<C: 'static + Send + Sync>(&self) -> &[MenuItem<C>] {
        self.menus
            .get(&std::any::TypeId::of::<C>())
            .and_then(|items| items.downcast_ref::<Vec<MenuItem<C>>>())
            .map(|vec| vec.as_slice())
            .unwrap_or_default()
    }

    /// 获取指定上下文类型的所有菜单项的可变引用
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 返回值
    /// 返回指定类型的菜单项可变切片，如果没有则返回空切片
    pub fn get_items_mut<C: 'static + Send + Sync>(&mut self) -> &mut [MenuItem<C>] {
        self.menus
            .get_mut(&std::any::TypeId::of::<C>())
            .and_then(|items| items.downcast_mut::<Vec<MenuItem<C>>>())
            .map(|vec| vec.as_mut_slice())
            .unwrap_or_default()
    }

    /// 显示指定上下文类型的菜单
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `ui`: egui的UI上下文
    /// - `world`: Bevy的World引用
    /// - `context`: 上下文实例
    ///
    /// # 说明
    /// 此方法会自动构建菜单树结构并按层级显示所有菜单项
    pub fn show_menu<C: 'static + Send + Sync>(
        &mut self,
        ui: &mut Ui,
        world: &mut World,
        context: &C,
    ) {
        let items = self.get_items_mut::<C>();
        let mut tree = MenuTree::new(items);
        tree.render(ui, world, context);
    }
}

/// 菜单树结构，用于层级化渲染菜单
///
/// 根据菜单项的路径自动构建树形结构，支持子菜单嵌套
pub struct MenuTree<'a, C> {
    /// 菜单项列表的可变引用
    items: &'a mut [MenuItem<C>],
    /// 根节点，包含整个菜单树结构
    root: MenuNode,
}

/// 菜单节点枚举，定义树结构中的节点类型
enum MenuNode {
    /// 叶子节点，表示一个具体的菜单项
    ///
    /// 包含对应菜单项在items向量中的索引
    Item(usize),

    /// 子菜单节点，包含子菜单的标题和子节点映射
    SubMenu(String, IndexMap<String, MenuNode>),
}

impl<'a, C: 'static + Send + Sync> MenuTree<'a, C> {
    /// 创建一个新的菜单树
    ///
    /// # 参数
    /// - `items`: 菜单项列表的可变引用
    ///
    /// # 返回值
    /// 返回根据菜单项路径构建的菜单树结构
    pub fn new(items: &'a mut [MenuItem<C>]) -> Self {
        Self::generate_tree(items)
    }

    /// 生成菜单树结构
    ///
    /// 根据菜单项的路径自动构建层级化的树形结构
    ///
    /// # 参数
    /// - `items`: 菜单项列表的可变引用
    ///
    /// # 返回值
    /// 返回构建好的菜单树
    fn generate_tree(items: &'a mut [MenuItem<C>]) -> Self {
        let mut root = MenuNode::SubMenu(String::new(), IndexMap::new());

        // Helper function to build the entire tree structure
        fn build_recursive<C: 'static + Send + Sync>(
            items: &[MenuItem<C>],
            parent_path: &str,
            used_items: &mut std::collections::HashSet<usize>,
        ) -> IndexMap<String, MenuNode> {
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
                        children.insert(
                            item_name.to_string(),
                            MenuNode::SubMenu(item.title.to_string(), submenu_children),
                        );
                    }
                    _ => {
                        // This is a regular menu item
                        children.insert(item_name.to_string(), MenuNode::Item(index));
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

        Self { items, root }
    }

    /// 渲染整个菜单树
    ///
    /// # 参数
    /// - `ui`: egui的UI上下文
    /// - `world`: Bevy的World引用
    /// - `context`: 上下文实例
    pub fn render(&mut self, ui: &mut Ui, world: &mut World, context: &C) {
        Self::render_recursive(&mut self.root, &mut self.items, ui, world, context, true);
    }

    /// 递归渲染菜单树节点
    ///
    /// # 参数
    /// - `node`: 当前要渲染的菜单节点
    /// - `items`: 菜单项列表的可变引用
    /// - `ui`: egui的UI上下文
    /// - `world`: Bevy的World引用
    /// - `context`: 上下文实例
    ///
    /// # 说明
    /// 根据节点类型决定渲染方式：
    /// - Item节点：渲染为按钮或可点击菜单项
    /// - SubMenu节点：渲染为子菜单，并递归渲染其子节点
    fn render_recursive(
        node: &mut MenuNode,
        items: &mut [MenuItem<C>],
        ui: &mut Ui,
        world: &mut World,
        context: &C,
        is_root: bool,
    ) {
        match node {
            MenuNode::Item(index) => {
                let item = &mut items[*index];
                let visible = item.when.run_readonly((), world);

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
                    Action::Custom(reflect_system_id) => {
                        world.resource_scope(
                            |world, mut registry: Mut<crate::reflect_system::RSystemRegistry>| {
                                if let Err(e) = registry.run_instant(
                                    reflect_system_id,
                                    (InMut(ui), InRef(context)),
                                    world,
                                ) {
                                    error!(
                                        "Failed to run custom menu {}: {}",
                                        reflect_system_id, e
                                    );
                                }
                            },
                        );
                    }
                    Action::SubMenu => {
                        // SubMenu items are handled by the tree structure
                    }
                }
            }
            MenuNode::SubMenu(ref title, ref mut children) => {
                if !children.is_empty() {
                    // Sort children by priority using indices
                    let mut sorted_children: Vec<_> = children.iter_mut().collect();
                    sorted_children.sort_by_key(|(_, node)| match **node {
                        MenuNode::Item(index) => items[index].priority,
                        MenuNode::SubMenu(_, _) => 0,
                    });
                    if is_root {
                        for (_, child) in sorted_children {
                            Self::render_recursive(child, items, ui, world, context, false);
                        }
                    } else {
                        ui.menu_button(&*title, |ui| {
                            for (_, child) in sorted_children {
                                Self::render_recursive(child, items, ui, world, context, false);
                            }
                        });
                    }
                }
            }
        }
    }
}

/// 菜单注册trait，为App提供便捷的菜单注册方法
///
/// 此trait提供了多种注册菜单项的方式，支持子菜单、命令和自定义系统的注册
pub trait MenuRegistration {
    /// 注册一个子菜单
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `path`: 子菜单路径（例如："文件/新建"）
    /// - `id`: 子菜单的唯一标识符
    /// - `title`: 子菜单的显示文本
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    ///
    /// # 示例
    /// ```
    /// app.register_submenu::<()>("文件", "file_menu", "文件");
    /// ```
    fn register_submenu<C>(
        &mut self,
        path: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
    ) -> &mut Self
    where
        C: 'static + Send + Sync;

    /// 注册一个菜单项
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `item`: 要注册的菜单项
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    ///
    /// # 示例
    /// ```
    /// let item = MenuItem::new("save", "保存", "文件/保存", Action::Command("save_file".into(), PhantomData::<()>));
    /// app.register(item);
    /// ```
    fn register<C: 'static + Send + Sync>(&mut self, item: MenuItem<C>) -> &mut Self;

    /// 注册一个命令菜单项
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `path`: 菜单项路径（例如："文件/保存"）
    /// - `id`: 菜单项的唯一标识符
    /// - `title`: 菜单项的显示文本
    /// - `command`: 要执行的命令的ActionId
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    ///
    /// # 示例
    /// ```
    /// app.register_command::<()>("文件/保存", "save", "保存", "save_file");
    /// ```
    fn register_command<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        command: impl Into<ActionId>,
    ) -> &mut Self;

    /// 注册一个自定义系统菜单项
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `path`: 菜单项路径（例如："文件/自定义"）
    /// - `id`: 菜单项的唯一标识符
    /// - `title`: 菜单项的显示文本
    /// - `system_id`: 自定义系统的标识符
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    ///
    /// # 示例
    /// ```
    /// app.register_custom::<()>("文件/自定义", "custom_action", "自定义操作", "my_custom_system");
    /// ```
    fn register_custom<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        system_id: impl Into<crate::utils::identifier::Identifier>,
    ) -> &mut Self;
}

impl MenuRegistration for App {
    /// 注册一个菜单项到App
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `item`: 要注册的菜单项
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    fn register<C: 'static + Send + Sync>(&mut self, mut item: MenuItem<C>) -> &mut Self {
        self.world_mut()
            .resource_scope(|world, mut menu_system: Mut<MenuSystem>| {
                menu_system.register(item, world);
            });
        self
    }

    /// 注册一个命令菜单项到App
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `path`: 菜单项路径
    /// - `id`: 菜单项的唯一标识符
    /// - `title`: 菜单项的显示文本
    /// - `command`: 要执行的命令的ActionId
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    fn register_command<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        command: impl Into<ActionId>,
    ) -> &mut Self {
        self.register(MenuItem::new(
            title,
            path,
            Action::Command(command.into(), PhantomData::<C>),
        ))
    }

    /// 注册一个自定义系统菜单项到App
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `path`: 菜单项路径
    /// - `id`: 菜单项的唯一标识符
    /// - `title`: 菜单项的显示文本
    /// - `system_id`: 自定义系统的标识符
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    fn register_custom<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
        system_id: impl Into<crate::utils::identifier::Identifier>,
    ) -> &mut Self {
        let system_id = system_id.into();
        self.register(MenuItem::new(
            title,
            path,
            Action::Custom::<C>(system_id),
        ));

        self
    }

    /// 注册一个子菜单到App
    ///
    /// # 类型参数
    /// - `C`: 上下文类型，必须是'static + Send + Sync
    ///
    /// # 参数
    /// - `path`: 子菜单路径
    /// - `id`: 子菜单的唯一标识符
    /// - `title`: 子菜单的显示文本
    ///
    /// # 返回值
    /// 返回App的可变引用，支持链式调用
    fn register_submenu<C: 'static + Send + Sync>(
        &mut self,
        path: impl Into<String>,
        title: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        self.register(MenuItem::<C>::new(title, path, Action::SubMenu))
    }
}

/// 菜单系统插件，用于初始化菜单系统资源
///
/// 此插件会自动注册MenuSystem资源，使其在整个应用生命周期中可用
pub struct MenuSystemPlugin;

impl Plugin for MenuSystemPlugin {
    /// 构建插件，初始化菜单系统资源
    ///
    /// # 参数
    /// - `app`: Bevy的App引用，用于注册资源和系统
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuSystem>();
    }
}
