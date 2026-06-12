# Slint + Rust 音乐播放器 · 学习总纲

> 本文档是练习路线图，把 **Rust 基础** + **Slint GUI** 两边的知识点串在一起，按你项目的 6 个阶段编排。  
> 已有文档：`GUI_知识准备.md`（Rust 5 大知识点）、`Rust_多线程学习笔记.md`（多线程基础）。本文不再重复，改为**索引 + 补充 Slint 专属内容**。

---

## 目录

- [Part A — Rust 基础速查（已有文档索引）](#part-a--rust-基础速查已有文档索引)
- [Part B — Slint 入门：从 .slint 文件到第一个窗口](#part-b--slint-入门从-slint-文件到第一个窗口)
- [Part C — 数据在 Rust 与 Slint 之间流动](#part-c--数据在-rust-与-slint-之间流动)
- [Part D — Slint 控件速查（播放器会用到的）](#part-d--slint-控件速查播放器会用到的)
- [Part E — Slint 事件循环与 Rust 多线程协作](#part-e--slint-事件循环与-rust-多线程协作)
- [Part F — 阶段练习映射（每一步练什么）](#part-f--阶段练习映射每一步练什么)
- [附录：关键 API 速记卡片](#附录关键-api-速记卡片)

---

## Part A — Rust 基础速查（已有文档索引）

以下内容已在 `GUI_知识准备.md` 和 `Rust_多线程学习笔记.md` 中详细讲解。这里只列索引，方便你定位：

| # | 知识点 | 哪份文档 | 章节 |
|---|--------|---------|------|
| 1 | `impl` 块：方法挂在结构体上 | GUI_知识准备.md | §1 |
| 2 | `Arc<Mutex<T>>`：多线程安全共享 | GUI_知识准备.md §2 + Rust_多线程 §5-7 |
| 3 | `move` 闭包：把变量所有权移入闭包 | GUI_知识准备.md §3 + Rust_多线程 §3 |
| 4 | 枚举状态机：`match` + `*self` 解引用 | GUI_知识准备.md §4 |
| 5 | 通道 `mpsc::channel`：线程间传消息 | GUI_知识准备.md §5 + Rust_多线程 §8-9 |
| 6 | `Rc` vs `Arc` 区别 | Rust_多线程学习笔记.md §4-5、§13 |
| 7 | 综合 Demo：5 个知识点串一起 | GUI_知识准备.md §6 |

> **建议**：先把 `GUI_知识准备.md` 的 §6 Demo 跑到能输出预期结果，再进入 Slint 部分。

---

## Part B — Slint 入门：从 .slint 文件到第一个窗口

### B.1 `.slint` 文件是什么

Slint 用 `.slint` 文件描述 UI（类似 QML / HTML+CSS 的声明式语言）。编译器自动生成 Rust 绑定代码。

```
hello.slint  ──(slint::slint! 宏或 build.rs)──▶  一个 Rust 类型，可以直接 ::new() 调用
```

**最小示例项目结构：**

```
hello-slint/
├── Cargo.toml
├── src/
│   └── main.rs
└── ui/
    └── app.slint
```

`Cargo.toml` 依赖：

```toml
[dependencies]
slint = "1.16.1"
```

### B.2 第一个 .slint 文件

```slint
// ui/app.slint
export component App inherits Window {
    title: "Hello Slint";

    VerticalBox {
        Text {
            text: "你好，这是我的第一个 Slint 窗口";
            font-size: 24px;
        }
        Button {
            text: "点我";
            clicked => {
                // 回调：按钮被点击时触发
            }
        }
    }
}
```

### B.3 Rust 侧启动窗口

```rust
// src/main.rs
slint::slint! {
    import { App } from "ui/app.slint";
}

fn main() -> Result<(), slint::PlatformError> {
    let app = App::new()?;
    app.run()
}
```

> `slint::slint!` 宏在编译期把 `.slint` 转成 Rust 类型。也可以配置 `build.rs` 由 `slint-build` 单独处理，两种方式等价。

### B.4 你这时候该练什么

- [ ] 建一个独立项目，把 B.2 + B.3 跑起来
- [ ] 改窗口 `title`，加几个 `Text`，换 `HorizontalBox`
- [ ] 点按钮看能不能触发 `clicked => { ... }`（先用纯 Slint 侧逻辑，比如 `debug("clicked")`）
- [ ] 体会 Slint 的布局规则：`VerticalBox` 上下堆 / `HorizontalBox` 左右排

---

## Part C — 数据在 Rust 与 Slint 之间流动

这是整个 GUI 编程的核心：**Rust 有数据，Slint 要显示它；用户动了 Slint 界面，Rust 要知道。**

### C.1 Property（属性）—— Rust 写 Slint 读

**Slint 侧声明：**

```slint
export component App inherits Window {
    in-out property <string> status-text: "就绪";
    //                   ^^^^^^^              ^^^^
    //                   类型                  默认值

    Text {
        text: status-text;   // ← 绑定到 property
    }
}
```

**Rust 侧修改：**

```rust
let app = App::new()?;
app.set_status_text("正在扫描文件...".into());
//  ↑ Slint 自动生成 set_<property-name>() 方法
```

> **命名规则**：Slint 的 `property <type> my-name` → Rust 的 `set_my_name(val)` / `get_my_name()`。

### C.2 Callback（回调）—— Slint 触发 Rust 执行

**Slint 侧声明：**

```slint
export component App inherits Window {
    callback play-button-clicked();
    //       ↑ 无参数回调

    Button {
        text: "▶ 播放";
        clicked => { play-button-clicked(); }
    }
}
```

**Rust 侧绑定：**

```rust
let app = App::new()?;
app.on_play_button_clicked(move || {
    // ← move 把外部变量移入（参考 GUI_知识准备 §3）
    println!("播放按钮被点击！");
    // 这里调用播放逻辑……
});
```

> **命名规则**：Slint 的 `callback my-callback(type)` → Rust 的 `on_my_callback(closure)`。

### C.3 Callback 带参数

```slint
callback song-selected(int);   // 接收歌曲索引
```

```rust
app.on_song_selected(move |idx: i32| {
    println!("用户选了第 {} 首", idx);
});
```

### C.4 双向绑定 `<=>`

```slint
Slider {
    min: 0;
    max: 100;
    value <=> root.current-progress;  // Slider 拖动时自动同步 progress
}
```

Rust 侧读：

```rust
let progress = app.get_current_progress();  // 实时获取
```

### C.5 你这时候该练什么

- [ ] 在 `.slint` 里加一个 `property` 和一个 `Text` 绑定它
- [ ] Rust 侧用 `set_xxx()` 改 property，看 UI 是否更新
- [ ] 加一个 `Button` + `callback`，Rust 侧 `on_xxx()` 绑定
- [ ] 加一个带参数的 callback（比如把 `LineEdit` 的 `text` 传给 Rust）

---

## Part D — Slint 控件速查（播放器会用到的）

### D.1 布局容器

| 控件 | 作用 | 播放器用法 |
|------|------|-----------|
| `VerticalBox` | 垂直排列子组件 | 整体页面：顶部工具栏 + 中间列表 + 底部控制栏 |
| `HorizontalBox` | 水平排列子组件 | 底部按钮栏（上一首 / 播放 / 下一首） |
| `GridLayout` | 网格布局 | 歌曲列表的表头（序号 | 歌名 | 时长） |

```slint
VerticalBox {
    // 顶部
    HorizontalBox {
        Text { text: "曲库"; font-size: 20px; }
    }
    // 中间（歌曲列表）
    // ...
    // 底部
    HorizontalBox {
        Button { text: "◀"; }
        Button { text: "▶"; }
        Button { text: "▶▶"; }
    }
}
```

### D.2 文本与按钮

| 控件 | 关键属性 / 回调 | 
|------|----------------|
| `Text` | `text` (输出)、`font-size`、`color`、`horizontal-alignment` |
| `Button` | `text`、`clicked => { }`、`enabled` |

### D.3 列表 —— 播放器最核心的控件

**方式一：`VerticalBox` 里循环（数据简单时）**

```slint
for song in root.songs: VerticalBox {
    HorizontalBox {
        Text { text: song.title; }
    }
}
```

**方式二：`ListView`（数据多 / 需要滚动）**

```slint
ListView {
    for song in root.songs: Rectangle {
        height: 40px;
        Text {
            text: song.title;
        }
    }
}
```

### D.4 输入控件

| 控件 | 关键属性 |
|------|---------|
| `LineEdit` | `text` (读写)、`placeholder-text`、`accepted => { }`（回车） |
| `Slider` | `value`、`min`、`max`（进度条） |
| `CheckBox` | `checked` |

```slint
// 搜索框
LineEdit {
    placeholder-text: "搜索歌曲…";
    accepted => { root.search(text); }
}

// 进度条
Slider {
    min: 0;
    max: 100;
    value <=> root.progress;
}
```

### D.5 你这时候该练什么

- [ ] 用 `ListView` 显示一个硬编码的歌曲列表（3-5 首歌）
- [ ] 每行显示序号 + 歌名
- [ ] 加 `LineEdit` 搜索框，输入文字后 Slint 侧过滤列表（纯 Slint 逻辑，暂不涉及 Rust）
- [ ] 尝试 `Slider` 双向绑定一个 property

---

## Part E — Slint 事件循环与 Rust 多线程协作

> ⚠️ 这是最容易卡住的部分。先确保 `GUI_知识准备.md` §5（通道）和 §6（综合 Demo）已经完全理解。

### E.1 Slint 的 `run()` 会阻塞当前线程

```rust
fn main() {
    let app = App::new().unwrap();
    app.run();  // ← 这里会一直运行事件循环，不会返回
    // 下面的代码永远不会执行
}
```

这意味着：**所有耗时操作（播放、扫描、网络请求）必须跑在其他线程里。**

### E.2 定时器：`slint::Timer`

Slint 内置定时器，用于周期性更新 UI（比如播放进度条）。

```rust
use slint::Timer;

let app_weak = app.as_weak();  // 弱引用，避免循环引用
let timer = Timer::default();
timer.start(
    slint::TimerMode::Repeated,   // 重复触发
    std::time::Duration::from_millis(200), // 每 200ms
    move || {
        let app = app_weak.upgrade().unwrap();  // 升级为强引用
        // 读取播放器进度，更新 Slint property
        app.set_progress(new_progress);
        // …
    },
);
```

> **重要**：闭包里用 `app.as_weak()` 而非直接 clone `app`。如果窗口关闭但 timer 还活着，`upgrade()` 返回 `None`，安全退出，不会崩溃。

### E.3 从其他线程安全地更新 Slint UI：`invoke_from_event_loop`

当你的播放线程想要通知 UI（比如"播放完毕，切歌"），**不能直接**调 `app.set_xxx()`，因为 Slint 不是 thread-safe 的。必须用：

```rust
use slint::invoke_from_event_loop;

let app_weak = app.as_weak();
std::thread::spawn(move || {
    // 播放完毕……
    let _ = invoke_from_event_loop(move || {
        let app = app_weak.upgrade().unwrap();
        app.set_now_playing(next_song_title.into());
    });
});
```

> `invoke_from_event_loop` 把一段代码"投递"到 Slint 的事件循环所在线程执行，安全地更新 UI。

### E.4 完整模式：播放线程 + 通道 + invoke_from_event_loop

这是你最终播放器的架构骨架：

```
┌────────────────────────────────────────────┐
│  主线程 (Slint 事件循环)                     │
│                                            │
│  app.run()  ← 永久循环处理 UI 事件           │
│    ├── 定时器 (Timer)                       │
│    │   └── 读进度，更新进度条 Slider         │
│    │                                       │
│    └── invoke_from_event_loop()            │
│        └── 其他线程通过它投递 UI 更新        │
└────────────────────────────────────────────┘
         ▲                    │
         │  mpsc::channel     │ spawn
         │                    ▼
┌────────────────────────────────────────────┐
│  播放线程                                   │
│  ┌──────────────────────────────────┐      │
│  │ rodio Sink / Player              │      │
│  │   → sleep_until_end() 阻塞等待   │      │
│  │   → 播完发 SongFinished 到 channel│      │
│  └──────────────────────────────────┘      │
└────────────────────────────────────────────┘
```

**关键代码片段：**

```rust
use std::sync::{Arc, Mutex, mpsc};
use slint::{Timer, TimerMode, invoke_from_event_loop};

enum PlayerEvent {
    SongFinished,
    Tick(f32), // 播放进度(秒)
}

fn main() -> Result<(), slint::PlatformError> {
    let app = App::new()?;
    let (tx, rx) = mpsc::channel::<PlayerEvent>();
    let rx = Arc::new(Mutex::new(rx));  // 主线程定时轮询

    // ① 定时器轮询通道消息
    let app_weak = app.as_weak();
    let rx_timer = rx.clone();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        let rx = rx_timer.lock().unwrap();
        match rx.try_recv() {
            Ok(PlayerEvent::SongFinished) => {
                let app = app_weak.upgrade().unwrap();
                // 自动切下一首
            }
            Ok(PlayerEvent::Tick(pos)) => {
                let app = app_weak.upgrade().unwrap();
                app.set_progress(pos as f32);
            }
            Err(_) => {} // 无消息
        }
    });

    // ② 按钮回调 → 启动播放线程
    let lib: Arc<Mutex<Library>> = /* … */;
    let lib_for_play = lib.clone();
    let tx_for_play = tx.clone();
    app.on_play_button_clicked(move |idx: i32| {
        let lib = lib_for_play.lock().unwrap();
        let song = &lib.songs[idx as usize];
        let path = song.path.clone();
        drop(lib);

        let tx = tx_for_play.clone();
        std::thread::spawn(move || {
            // rodio 播放逻辑
            // 播完发 tx.send(PlayerEvent::SongFinished)
        });
    });

    // ③ 启动事件循环
    app.run()
}
```

### E.5 你这时候该练什么

- [ ] 把 `GUI_知识准备.md` §6 的 Demo 里的 `Library` 拆成 `Arc<Mutex<Library>>`，确认能边读边写
- [ ] 写一个独立小练习：Slint 窗口 + Timer（每 500ms 让一个数字自增，显示在 Text 里）
- [ ] 再写一个：按钮启动一个线程，线程 sleep 2 秒后用 `invoke_from_event_loop` 更新窗口文字
- [ ] 最后把 Timer + 通道 + invoke_from_event_loop 串起来

---

## Part F — 阶段练习映射（每一步练什么）

> 对应之前给出的 6 个 Phase。这里列出每阶段具体练的条目，完成一个勾一个。

### Phase 0：环境准备

- [ ] `cargo check` 确认 slint 能编译
- [ ] `cargo run -- list` / `cargo run -- play` 跑通现有 CLI

### Phase 1：最小 Slint 窗口

- [ ] 建独立练习项目（或直接在 `src/ui/` 下开始）
- [ ] 写 `app.slint`：窗口 + 标题 + 几个 `Text`
- [ ] `main.rs` 用 `slint::slint!` 宏引入，`::new()` + `.run()`
- [ ] 加 `Button` + `callback`，Rust 侧 `on_xxx()` 绑定
- [ ] 加 `property`，Rust 侧 `set_xxx()` 改值看 UI 刷新

### Phase 2：集成到项目 — 显示歌单

- [ ] 在 `app.slint` 里用 `ListView` 或 `for-in` 循环显示歌曲列表
- [ ] Rust 侧定义数据模型（`slint::VecModel` 或 `SharedString` 序列）
- [ ] 从 `Library` 加载数据，喂给 Slint 控件
- [ ] 列表项点击回调：`on_song_selected(idx)`

### Phase 3：播放控制

- [ ] 底部控制栏：播放/暂停、上一首、下一首按钮
- [ ] `PlaybackState` 枚举（参考 `GUI_知识准备.md` §4）
- [ ] 按钮回调 → Rust 侧调用 rodio 播放 / `sink.pause()` / `sink.stop()`
- [ ] Timer 定时更新进度条（`Slider` 双向绑定）

### Phase 4：多线程协作

- [ ] 把 rodio 播放逻辑放进 `std::thread::spawn`
- [ ] 播放线程通过 `mpsc::channel` 发送 `SongFinished` / `Tick` 事件
- [ ] 主线程 Timer 轮询 `rx.try_recv()`，处理切歌
- [ ] 使用 `invoke_from_event_loop` 确保 UI 更新线程安全

### Phase 5：打磨 UI

- [ ] 自定义颜色主题（`color`、`background`、`border-radius`）
- [ ] `LineEdit` 搜索框 + 实时过滤列表
- [ ] 键盘快捷键（空格=播放/暂停，方向键切歌）
- [ ] 滚动列表优化（`Flickable`）

### Phase 6：可选进阶

- [ ] 拆分 `.slint` 文件（`Header.slint`、`Playlist.slint`、`PlayerUI.slint`）
- [ ] GUI 内触发 `scan` 文件夹、保存 `library.json`
- [ ] 支持从文件对话框选择音乐文件夹（`slint::api::open_file_dialog()`）

---

## 附录：关键 API 速记卡片

### Slint 侧 → Rust 侧命名规则

| Slint 声明 | Rust 方法 |
|-----------|----------|
| `property <int> my-val` | `set_my_val(i32)` / `get_my_val() -> i32` |
| `callback my-action(string)` | `on_my_action(impl Fn(SharedString) + 'static)` |
| `in property <int> x` | `set_x()` 可用（输入属性） |
| `out property <int> y` | 只读，Slint 内部算出来的 |
| `in-out property <int> z` | `set_z()` + `get_z()` 都可用 |

### Slint 类型 ↔ Rust 类型

| Slint 类型 | Rust 类型 |
|-----------|----------|
| `int` | `i32` |
| `float` | `f32` |
| `string` | `slint::SharedString`（可以 `.into()` 转 `String`） |
| `bool` | `bool` |
| `duration` | `i64`（毫秒） |
| `color` | `slint::Color` |
| `image` | `slint::Image` |
| `[int]` | `slint::VecModel<i32>` |

### 常用 Rust API

```rust
// 创建窗口
let app = App::new()?;

// 弱引用（用于 Timer / 线程闭包）
let weak = app.as_weak();
let app = weak.upgrade().unwrap();  // 可能返回 None

// 定时器
use slint::{Timer, TimerMode};
let timer = Timer::default();
timer.start(TimerMode::Repeated, Duration::from_millis(200), move || { /* ... */ });
// TimerMode::Single 只触发一次

// 从其他线程安全更新 UI
slint::invoke_from_event_loop(move || {
    // 这里面可以直接调 app.set_xxx()
}).ok();

// VecModel（动态列表）
use slint::VecModel;
let model: slint::ModelRc<SongData> = slint::VecModel::default().into();
model.push(SongData { title: "晴天".into() });
app.set_songs(model.into());  // 传给 Slint 的 property <[SongData]>

// 文件对话框
use slint::api::OpenFileDialog;
let result = slint::api::open_file_dialog("选择音乐文件夹", &["*"]);
```

---

## 学习节奏建议

| 周 | 内容 |
|----|------|
| 1 | `GUI_知识准备.md` 读一遍 + §6 Demo 跑通 |
| 2 | Phase 1：最小 Slint 窗口（独立练习项目） |
| 3 | Phase 2：把现有项目的歌单显示到 Slint 里 |
| 4-5 | Phase 3 + 4：播放控制 + 多线程（最难点，多花时间） |
| 6 | Phase 5：打磨 UI |

> **关键原则**：每做一步都 `cargo run` 验证，不要攒到最后一起调。Slint 的编译错误信息很有用，但它和 Rust 的错误混在一起需要习惯。

---

*文档生成时间：2025-07-12*
