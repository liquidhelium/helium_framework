# Helium Framework
An editor framework for bevy using egui.

## Features

* Convenient docking, register a system with `In<Ui>` and you are ready to go.
* Flexible and configurable menu items.
* Add an hotkey with just a line.

```rust
app.register_hotkey("maximize", [Hotkey::new_global([KeyCode::ControlLeft, KeyCode::KeyM])]);
```

* Register bevy systems as `Action`s, so that they can be called dynamically. For example, to use them as what a hotkey triggers.