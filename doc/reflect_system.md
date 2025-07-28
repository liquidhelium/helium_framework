# Reflect System 使用文档

Reflect System 是 Helium Framework 的核心组件，提供了基于反射的系统注册和调用机制。它允许你将任何 Bevy 系统注册为可动态调用的"动作"，并支持类型安全的参数传递。

## 核心概念

### ActionId
- 系统的唯一标识符，使用 `Identifier` 类型
- 可以是字符串或其他实现了 `From<Identifier>` 的类型

### 系统注册
- 将任意 Bevy 系统包装成可反射调用的动作
- 支持多种输入类型：`In<T>`, `InRef<T>`, `InMut<T>` 以及它们的元组组合
- 自动处理类型检查和转换

## 基本用法

### 1. 添加插件

```rust
use bevy::prelude::*;
use helium_framework::reflect_system::ActionPlugin;

let mut app = App::new();
app.add_plugins(ActionPlugin);
```

### 2. 注册系统

使用 `reflect_system` 方法注册系统。任何合法的 Bevy 系统都可以被注册，包括使用 `Res`, `ResMut`, `Local` 等系统参数：

```rust
use helium_framework::reflect_system::ActionsExt;

// 注册一个简单的系统
app.reflect_system(
    "double_value",           // ActionId
    "将输入值乘以2",          // 描述
    |In(input): In<i32>| {    // 系统函数
        input * 2
    }
);

// 注册使用 Bevy 系统参数的系统
app.reflect_system(
    "process_with_resource",
    "使用资源处理输入",
    |In(input): In<String>, config: Res<AppConfig>| -> String {
        format!("{} - {}", input, config.app_name)
    }
);

// 注册修改资源的系统
app.reflect_system(
    "update_counter",
    "更新计数器资源",
    |In(delta): In<i32>, mut counter: ResMut<Counter>| {
        counter.0 += delta;
    }
);

// 注册带引用的系统
app.reflect_system(
    "process_reference",
    "处理引用参数",
    |InRef(input): InRef<String>| {
        println!("Input: {}", input);
        input.len()
    }
);

// 注册带可变引用的系统
app.reflect_system(
    "modify_value",
    "修改传入的值",
    |InMut(input): InMut<i32>| {
        *input += 10;
    }
);
```

### 3. 多参数系统

支持元组形式的多个参数：

```rust
app.reflect_system(
    "complex_operation",
    "复杂的多参数操作",
    |(In(value), InRef(config), InMut(result)): (In<i32>, InRef<Config>, InMut<i32>)| {
        *result = value * config.multiplier;
        *result + config.offset
    }
);
```

### 4. 运行系统

#### 即时运行（阻塞式）
即时运行不需要提供 `'static` 的引用，适合测试和初始化场景：

```rust
use helium_framework::reflect_system::{RSystemRegistry, ActionError};

app.world_mut().resource_scope(|world, mut registry: Mut<RSystemRegistry>| {
    // 运行无返回值的系统
    registry.run_instant::<In<i32>>(
        &"double_value".into(),
        In(42),
        world
    ).unwrap();
    
    // 运行有返回值的系统
    let result = registry.run_instant_ret::<In<i32>, i32>(
        &"double_value".into(),
        In(21),
        world
    ).unwrap();
    assert_eq!(result, 42);
    
    // 使用引用参数（无需'static）
    let local_string = "Hello".to_string();
    let length = registry.run_instant_ret::<InRef<String>, usize>(
        &"process_reference".into(),
        InRef(&local_string),
        world
    ).unwrap();
    assert_eq!(length, 5);
});
```

#### 延迟运行（非阻塞式）
延迟运行需要 `'static` 的引用，适合在游戏循环中调用：

```rust
use helium_framework::reflect_system::Actions;

fn my_system(mut actions: Actions) {
    // 排队执行动作（需要'static输入）
    actions.run_action::<In<i32>>(
        &"double_value".into(),
        In(100)  // i32实现了'static
    ).unwrap();
    
    // 错误处理
    if let Err(e) = actions.run_action::<In<i32>>(
        &"non_existent".into(), 
        In(0)
    ) {
        eprintln!("运行失败: {:?}", e);
    }
}

// 注意：不能使用非'static的引用
// 错误示例：
fn bad_example(mut actions: Actions) {
    let local_string = "temp".to_string();
    // 编译错误：local_string没有'static生命周期
    // actions.run_action::<InRef<String>>(
    //     &"process_reference".into(),
    //     InRef(&local_string)
    // ).unwrap();
}
```

## 输入类型支持

### 支持的输入包装器

| 包装器 | 描述 | 用法示例 | 生命周期要求 |
|--------|------|----------|-------------|
| `In<T>` | 按值传递 | `In(42)` | T: 'static |
| `InRef<T>` | 不可变引用 | `InRef(&value)` | T: 'static |
| `InMut<T>` | 可变引用 | `InMut(&mut value)` | T: 'static |

### 元组组合

支持最多8个参数的元组组合：

```rust
// 单参数
In<i32>

// 多参数
(In<i32>, InRef<String>, InMut<bool>)

// 更多参数（最多8个）
(In<i32>, InRef<String>, InMut<bool>, In<f64>, InRef<Vec<u8>>)
```

### 生命周期注意事项

- **即时运行** (`run_instant`, `run_instant_ret`)：不需要 `'static` 生命周期，可以使用局部变量
- **延迟运行** (`run_action`)：需要 `'static` 生命周期，只能使用实现了 `'static` 的类型

```rust
// 即时运行 - 可以使用局部变量
fn immediate_example(world: &mut World) {
    let local_data = vec![1, 2, 3];
    world.resource_scope(|world, mut registry: Mut<RSystemRegistry>| {
        registry.run_instant_ret::<InRef<Vec<i32>>, usize>(
            &"process_vec".into(),
            InRef(&local_data),
            world
        ).unwrap();
    });
}

// 延迟运行 - 只能使用'static数据
fn deferred_example(mut actions: Actions) {
    // 正确：使用'static数据
    let static_data = Arc::new(vec![1, 2, 3]);
    actions.run_action::<In<Arc<Vec<i32>>>(
        &"process_arc".into(),
        In(static_data)
    ).unwrap();
    
    // 错误：不能使用局部变量的引用
    // let local_data = vec![1, 2, 3];
    // actions.run_action::<InRef<Vec<i32>>(
    //     &"process_vec".into(),
    //     InRef(&local_data)
    // ).unwrap();
}
```

## 错误处理

### 错误类型

```rust
use helium_framework::reflect_system::ActionError;

match result {
    Ok(output) => println!("成功: {}", output),
    Err(ActionError::NotFound { id }) => {
        eprintln!("动作 '{}' 未找到", id);
    }
    Err(ActionError::MismatchInput { expected_type_name, found_type_name }) => {
        eprintln!("类型不匹配: 期望 {}, 实际 {}", expected_type_name, found_type_name);
    }
    Err(ActionError::RegistrationError { message }) => {
        eprintln!("注册错误: {}", message);
    }
}
```

### 类型验证

```rust
app.world_mut().resource_scope(|_world, registry: Mut<RSystemRegistry>| {
    // 验证输入输出类型是否匹配
    registry.verify_type::<In<i32>, i32>(&"double_value".into()).unwrap();
    
    // 验证失败会返回 MismatchInput 错误
    registry.verify_type::<In<String>, String>(&"double_value".into()).unwrap_err();
});
```

## 高级用法

### 获取系统元数据

```rust
app.world_mut().resource_scope(|_world, registry: Mut<RSystemRegistry>| {
    if let Some(meta) = registry.get_meta(&"double_value".into()) {
        println!("系统描述: {}", meta.description);
        println!("输入类型: {}", meta.input);
        println!("输出类型: {}", meta.output);
    }
});
```

### 动态系统调用

```rust
// 从配置或用户输入中动态构建调用
fn dynamic_call(registry: &mut RSystemRegistry, action_name: &str, input: i32) -> Option<i32> {
    let action_id: helium_framework::utils::identifier::Identifier = action_name.into();
    registry.run_instant_ret::<In<i32>, i32>(&action_id, In(input)).ok()
}
```

## 完整示例

```rust
use bevy::prelude::*;
use helium_framework::reflect_system::{ActionPlugin, ActionsExt, Actions};
use std::sync::Arc;

#[derive(Resource)]
struct Counter(i32);

#[derive(Resource)]
struct AppConfig {
    app_name: String,
}

fn setup_system(mut commands: Commands) {
    commands.insert_resource(Counter(0));
    commands.insert_resource(AppConfig {
        app_name: "Helium Framework".to_string(),
    });
}

fn main() {
    let mut app = App::new();
    app.add_plugins(ActionPlugin)
       .add_systems(Startup, setup_system);

    // 注册各种系统，展示完整的 Bevy 系统支持
    app.reflect_system(
        "increment_counter",
        "增加计数器",
        |In(amount): In<i32>, mut counter: ResMut<Counter>| {
            counter.0 += amount;
        }
    );

    app.reflect_system(
        "get_counter",
        "获取计数器值",
        |counter: Res<Counter>| -> i32 {
            counter.0
        }
    );

    app.reflect_system(
        "process_with_config",
        "使用配置处理数据",
        |In(data): In<String>, config: Res<AppConfig>| -> String {
            format!("{} - {}", data, config.app_name)
        }
    );

    app.reflect_system(
        "complex_operation",
        "复杂操作示例",
        |In((value, text)): In<(i32, String)>, 
         mut counter: ResMut<Counter>,
         config: Res<AppConfig>| -> String {
            counter.0 += value;
            format!("Processed: {} - Counter: {} - App: {}", 
                    text, counter.0, config.app_name)
        }
    );

    // 测试即时运行（可以使用非'static引用）
    app.world_mut().resource_scope(|world, mut registry| {
        use helium_framework::reflect_system::RSystemRegistry;
        
        // 运行系统
        registry.run_instant::<In<i32>>(
            &"increment_counter".into(),
            In(5),
            world
        ).unwrap();
        
        let value = registry.run_instant_ret::<(), i32>(
            &"get_counter".into(),
            (),
            world
        ).unwrap();
        assert_eq!(value, 5);
        
        // 使用局部变量测试
        let test_string = "Hello World".to_string();
        let result = registry.run_instant_ret::<In<String>, String>(
            &"process_with_config".into(),
            In(test_string),
            world
        ).unwrap();
        assert!(result.contains("Hello World - Helium Framework"));
    });

    // 测试延迟运行（需要'static数据）
    app.add_systems(Update, |mut actions: Actions| {
        // 使用'static数据
        static_data = Arc::new("Test Data".to_string());
        actions.run_action::<In<Arc<String>>>(
            &"process_with_config".into(),
            In(static_data)
        ).unwrap();
    });
}
```

## 注意事项

1. **生命周期管理**：
   - 即时运行：可以使用任意生命周期，包括局部变量引用
   - 延迟运行：必须使用 `'static` 生命周期，考虑使用 `Arc` 或克隆数据

2. **Bevy系统支持**：
   - 任何合法的 Bevy 系统都可以注册为反射系统
   - 支持 `Res`, `ResMut`, `Local`, `Query`, `Commands` 等所有系统参数
   - 系统参数的顺序不影响功能，可以放在任何位置

3. **类型安全**：
   - 系统注册时会保存类型信息，调用时会进行类型检查
   - 使用 `verify_type` 方法可以在运行时验证类型兼容性

4. **性能考虑**：
   - 即时运行：阻塞式，适合初始化和测试
   - 延迟运行：非阻塞式，系统会在主线程排队执行
   - 频繁调用建议使用延迟运行以避免阻塞

5. **线程安全**：
   - 延迟运行的系统会在主线程执行，确保系统实现是线程安全的
   - 避免在系统中使用非线程安全的全局状态

6. **内存管理**：
   - 对于大对象，延迟运行时考虑使用 `Arc` 来共享数据
   - 即时运行时可以直接传递引用，避免不必要的克隆