# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Helium Framework is a Rust editor framework for Bevy using egui. It provides a modular architecture for building editor-like applications with docking, hotkeys, and menu systems.

## Build Commands

- **Build**: `cargo build`
- **Run example**: `cargo run --example basic`
- **Check**: `cargo check`
- **Test**: `cargo test`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## Architecture Overview

The framework consists of 5 main plugins that can be used independently or together:

### Core Plugins

1. **ActionPlugin** (`reflect_system.rs`) - Reflective system registry
   - Registers systems as actions with type-safe input/output
   - Provides `Actions` system parameter for running actions
   - Core concept: `ActionId` (alias for `Identifier`)

2. **HotkeyPlugin** (`hotkeys.rs`) - Global hotkey management
   - Supports multi-key combinations with different trigger types
   - Prevents hotkey activation during text input (unless modifiers used)
   - API: `app.register_hotkey("action_id", [Hotkey::new_global([KeyCode::Ctrl, KeyCode::KeyM])])`

3. **TabPlugin** (`tab_system.rs`) - Docking tab system
   - Uses `egui_dock` for window docking
   - Tabs are registered with `register_tab()` and rendered via systems
   - Manages tab availability through conditions

4. **MenuPlugin** (`menu.rs`) - Hierarchical menu system
   - Supports buttons, custom widgets, submenus, and categories
   - Priority-based ordering with `IndexMap`
   - API: `app.menu_context(|ctx| ctx.with_sub_menu("file", "File", 0, |ctx| ...))`

5. **NotificationPlugin** (`notifications.rs`) - Toast notifications
   - Uses `egui-notify` for in-app notifications

### Key Types

- `ActionId`/`TabId`: `Identifier` type for action/tab registration
- `InMut<'a, T>`: Wrapper for mutable references in tab systems
- `HeDockState`: Wrapper for `DockState<TabId>` for serialization
- `Identifier`: String-based ID type in `utils/identifier.rs`

## Usage Pattern

```rust
// Setup
app.add_plugins(HeliumFramework)
   .insert_resource(HeDockState(DockState::new(vec!["tab1".into()])));

// Register systems as actions
app.reflect_system("my_action", "description", my_system);

// Register tabs
app.register_tab("tab1", "My Tab", tab_ui, || true);

// Register hotkeys
app.register_hotkey("my_action", [Hotkey::new_global([KeyCode::KeyA])]);

// Setup menu
app.menu_context(|ctx| {
    ctx.with_sub_menu("file", "File", 0, |ctx| {
        ctx.add("quit", "Quit", Button::new("quit"), 0);
    });
});
```

## Dependencies

- **Bevy 0.16** - Game engine
- **bevy_egui 0.34** - egui integration
- **egui_dock 0.16** - Docking system
- **egui-notify 0.19** - Notifications
- **rust-i18n 3** - Internationalization
- **snafu 0.8** - Error handling

## Example Structure

The `basic.rs` example demonstrates all features:
- Tab registration and docking
- Hotkey system
- Menu system
- Action system with typed inputs/outputs