# 菜单系统迁移指南

本文档指导如何将旧的菜单系统（git 679e9d0版本）迁移到新的菜单系统。

## 概述

新菜单系统相比旧系统有以下主要改进：

- **类型安全**：使用泛型和类型系统确保上下文类型安全
- **简洁API**：更简单的注册和使用方式
- **性能优化**：减少了运行时的动态分发
- **更好的组织**：基于路径的层级结构更清晰

## 迁移对照表

| 旧系统（679e9d0） | 新系统 | 说明 |
|------------------|--------|------|
| `MenuPlugin` | `MenuSystemPlugin` | 插件名称变更 |
| `EditorMenuEntrys` | `MenuSystem` | 资源类型变更 |
| `menu_context()` | 无直接对应 | 新系统使用类型安全的注册方式 |
| `Button` | `Action::Command` | 按钮类型变为命令动作 |
| `Custom` | `Action::Custom` | 自定义类型变为自定义动作 |
| `SubMenu` | `Action::SubMenu` | 子菜单类型保持一致 |
| `Category` | 已移除 | 使用子菜单替代 |

## 迁移步骤

### 1. 插件变更

**旧代码：**
```rust
use helium_framework::menu::MenuPlugin;

app.add_plugins(MenuPlugin);
```

**新代码：**
```rust
use helium_framework::menu_system::MenuSystemPlugin;

app.add_plugins(MenuSystemPlugin);
```

### 2. 菜单注册方式变更

#### 2.1 基本菜单项注册

**旧代码：**
```rust
app.menu_context(|ctx| {
    ctx.with_sub_menu("file", "File", 0, |ctx| {
        ctx.add("new", "New", Button::new("file.new"), 0);
        ctx.add("open", "Open", Button::new("file.open"), 1);
    });
});
```

**新代码：**
```rust
use helium_framework::menu_system::*;

// 定义上下文类型
struct MainMenuContext;

app.register_submenu::<MainMenuContext>("file", "file", "文件")
    .register_command::<MainMenuContext>("file/new", "file.new", "新建", "file.new")
    .register_command::<MainMenuContext>("file/open", "file.open", "打开", "file.open");
```

#### 2.2 带条件的菜单项

**旧代码：**
```rust
use helium_framework::menu::Button;

ctx.add("save", "Save", Button::new_conditioned("file.save", |world: &World| {
    world.resource::<AppState>().has_unsaved_changes
}), 2);
```

**新代码：**
```rust
use helium_framework::menu_system::*;

let save_item = MenuItem::new("save", "保存", "file/save", Action::Command("file.save", PhantomData::<MainMenuContext>))
    .with_condition(|world: &World| world.resource::<AppState>().has_unsaved_changes)
    .with_priority(2);

app.register(save_item);
```

#### 2.3 自定义菜单项

**旧代码：**
```rust
use helium_framework::menu::Custom;

ctx.add("custom", "Custom Action", Custom::new(Box::new(|ui, world, name| {
    ui.label(format!("Custom: {}", name));
})), 0);
```

**新代码：**
```rust
// 注册自定义系统
app.reflect_system("custom.action", "Custom Action", |(InMut(ui), InRef(_ctx)): (InMut<Ui>, InRef<MainMenuContext>)| {
    ui.label("Custom Action");
});

// 注册到菜单
app.register_custom::<MainMenuContext>("custom", "custom.action", "自定义操作", "custom.action");
```

### 3. 菜单显示方式变更

#### 3.1 主菜单栏

**旧代码：**
```rust
use helium_framework::menu::show_menu_ui;

fn egui_main(world: &mut World) {
    let mut egui_context = world.query_filtered::<&mut EguiContext, With<PrimaryWindow>>();
    let mut binding = egui_context.single_mut(world).unwrap();
    let ctx = &binding.get_mut().clone();
    
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        show_menu_ui(ui, world);
    });
}
```

**新代码：**
```rust
use helium_framework::menu_system::*;

fn egui_main(world: &mut World) {
    let mut egui_context = world.query_filtered::<&mut EguiContext, With<PrimaryWindow>>();
    let mut binding = egui_context.single_mut(world).unwrap();
    let ctx = &binding.get_mut().clone();
    
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        world.resource_scope(|world, mut menu_system: Mut<MenuSystem>| {
            menu_system.show_menu::<MainMenuContext>(ui, world, &MainMenuContext);
        });
    });
}
```

#### 3.2 上下文菜单

**旧代码：**
没有对应的功能, 旧代码的一切都是为了主菜单栏

**新代码：**
```rust
// 定义专用的上下文类型
struct EditorContext;

// 注册上下文菜单项
app.register_command::<EditorContext>("copy", "editor.copy", "复制", "editor.copy")
    .register_command::<EditorContext>("paste", "editor.paste", "粘贴", "editor.paste");

// 在UI元素上使用
ui.add(Label::new("右键点击我").sense(Sense::all()))
    .context_menu(|ui| {
        world.resource_scope(|world, mut menu_system: Mut<MenuSystem>| {
            menu_system.show_menu::<EditorContext>(ui, world, &EditorContext);
        });
    });
```

### 4. 路径和ID变更

- **路径格式**：从点分格式（`"file.new"`）变为斜杠格式（`"file/new"`）
- **ID格式**：保持相同，但推荐使用更具描述性的ID
- **优先级**：从`usize`变为`i32`，值越小优先级越高

### 5. 条件函数变更

**旧系统**：使用`Condition<M>` trait
**新系统**：使用`Condition<M>` trait，与hotkeys系统保持一致

## 完整迁移示例

### 旧系统完整示例

```rust
use bevy::prelude::*;
use helium_framework::menu::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(HeliumFramework)
        .add_plugins(MenuPlugin)
        .menu_context(|ctx| {
            ctx.with_sub_menu("file", "File", 0, |ctx| {
                ctx.add("new", "New", Button::new("file.new"), 0);
                ctx.add("open", "Open", Button::new("file.open"), 1);
                ctx.add("save", "Save", Button::new_conditioned("file.save", |world| {
                    world.resource::<AppState>().has_unsaved_changes
                }), 2);
                ctx.add("quit", "Quit", Button::new("file.quit"), 3);
            });
            ctx.with_sub_menu("edit", "Edit", 1, |ctx| {
                ctx.add("undo", "Undo", Button::new("edit.undo"), 0);
                ctx.add("redo", "Redo", Button::new("edit.redo"), 1);
            });
        });
    
    // ... 其他代码
}
```

### 新系统完整示例

```rust
use bevy::prelude::*;
use helium_framework::menu_system::*;
use std::marker::PhantomData;

#[derive(Debug)]
struct MainMenuContext;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(HeliumFramework)
        .add_plugins(MenuSystemPlugin)
        .register_submenu::<MainMenuContext>("file", "file", "文件")
        .register_command::<MainMenuContext>("file/new", "file.new", "新建", "file.new")
        .register_command::<MainMenuContext>("file/open", "file.open", "打开", "file.open")
        .register_command::<MainMenuContext>("file/save", "file.save", "保存", "file.save")
        .register_command::<MainMenuContext>("file/quit", "file.quit", "退出", "file.quit")
        .register_submenu::<MainMenuContext>("edit", "edit", "编辑")
        .register_command::<MainMenuContext>("edit/undo", "edit.undo", "撤销", "edit.undo")
        .register_command::<MainMenuContext>("edit/redo", "edit.redo", "重做", "edit.redo");
    
    // 注册条件命令
    let save_item = MenuItem::new("save", "保存", "file/save", Action::Command("file.save", PhantomData::<MainMenuContext>))
        .with_condition(|world: &World| world.resource::<AppState>().has_unsaved_changes)
        .with_priority(2);
    app.register(save_item);
    
    // ... 其他代码
}
```

## 迁移检查清单

- [ ] 替换 `MenuPlugin` 为 `MenuSystemPlugin`
- [ ] 移除 `menu_context()` 调用
- [ ] 定义上下文类型（如 `MainMenuContext`, 旧系统的一切都是为了MainMenu的）
- [ ] 将 `Button::new()` 改为 `Action::Command`
- [ ] 将 `Button::new_conditioned()` 改为 `.with_condition()`
- [ ] 将 `Custom::new()` 改为 `Action::Custom`
- [ ] 将路径格式从点分改为斜杠格式
- [ ] 更新菜单显示代码
- [ ] 测试所有菜单功能

## 常见问题

### Q: 如何同时支持多个不同的菜单上下文？

**A:** 为不同的场景定义不同的上下文类型：

```rust
#[derive(Debug)]
struct MainMenuContext;
#[derive(Debug)]
struct EditorContext;
#[derive(Debug)]
struct DebugContext;

// 分别注册不同的菜单项
app.register_command::<MainMenuContext>(...)
   .register_command::<EditorContext>(...)
   .register_command::<DebugContext>(...);
```

### Q: 如何实现动态菜单项？

**A:** 使用 `.with_condition()` 方法：

```rust
let dynamic_item = MenuItem::new("item", "动态项", "path", Action::Command("action", PhantomData::<C>))
    .with_condition(|world: &World| {
        // 根据运行时状态决定是否显示
        world.resource::<AppState>().should_show_item
    });
```

### Q: 如何处理国际化？

**A:** 使用 `Cow<'static, str>` 支持的国际化方案：

```rust
use rust_i18n::t;

app.register_command::<MainMenuContext>("file/new", "file.new", t!("menu.file.new"), "file.new");
```

## 总结

新的菜单系统提供了更好的类型安全性和更简洁的API。虽然需要一些迁移工作，但长期来看会提高代码的可维护性和可读性。建议逐步迁移，先从简单的菜单开始，逐步替换复杂的菜单结构。