# Rust 音乐播放器工程化学习路线

这份文档不是为了让你一次性复制出完整播放器，而是为了让你知道每一步为什么要这样设计、每个文件负责什么、Rust 里会遇到哪些知识点。我们会按“先架构，后功能”的顺序来写。

项目第一阶段先使用 `crossterm` 做终端界面，播放层使用 `rodio 0.22.x`。后续迁移到 Slint GUI 时，尽量只替换 `ui` 层，不重写播放核心。

## 总体目标

第一阶段实现：

- 播放歌曲
- 暂停
- 继续播放
- 上一首
- 下一首
- 自动切歌

第二阶段预留：

- 播放进度显示
- 总时长显示
- 拖动进度条 Seek
- 音量控制
- 播放模式：顺序、单曲循环、随机

## 总体线程模型

核心原则：不要让 UI 线程阻塞。

```text
UI 线程
  读取键盘输入
  渲染终端界面
  更新 AppState
  发送 PlaybackCommand

        |
        v

播放线程
  接收 PlaybackCommand
  控制 rodio 播放
  检查歌曲是否播放结束
  发送 PlaybackEvent

        |
        v

UI 线程
  接收 PlaybackEvent
  更新 AppState
  重新渲染界面
```

这个结构的好处是：UI 不直接调用 `sleep_until_end()`，所以界面不会卡死。播放线程负责“真的播放”，UI 线程负责“显示和输入”。

## 为什么使用 Arc<Mutex<AppState>>

`AppState` 表示整个播放器当前状态，例如当前歌曲、播放状态、错误信息、播放模式等。

```rust
use std::sync::{Arc, Mutex};

pub type SharedAppState = Arc<Mutex<AppState>>;
```

为什么需要 `Arc`：

- `Arc` 是原子引用计数，可以让多个线程共同拥有同一份数据。
- UI 线程需要读写状态。
- 事件处理器需要更新状态。
- 后续 Slint 回调也可能需要访问状态。

为什么需要 `Mutex`：

- 多个地方可能修改 `AppState`。
- Rust 不允许多个线程随便同时修改同一份数据。
- `Mutex` 保证同一时刻只有一个地方能修改状态。

为什么不直接传引用：

- `thread::spawn` 启动的线程可能比当前函数活得更久。
- 普通引用有生命周期限制，不能随便跨线程长期保存。
- `Arc<Mutex<T>>` 是 Rust 中比较常见的“多线程共享可变状态”写法。

重要提醒：

- `AppState` 只放“应用状态”。
- 不要把 `rodio::Player`、`Sink`、音频设备这类底层播放对象塞进 `AppState`。
- 播放器对象应该由播放线程自己拥有，这样职责更清楚。

## 第一步：设计项目目录结构

目标：先让每个模块有明确职责，避免 `main.rs` 或 `lib.rs` 变成巨型文件。

推荐结构：

```text
src/
  main.rs
  lib.rs
  app.rs

  models/
    mod.rs
    song.rs
    library.rs

  state/
    mod.rs
    app_state.rs
    playback_state.rs

  commands.rs
  events.rs

  playback/
    mod.rs
    engine.rs
    rodio_player.rs

  ui/
    mod.rs
    terminal.rs
    render.rs
    input.rs

  handlers/
    mod.rs
    playback_handler.rs
    event_handler.rs
```

每个模块负责什么：

- `main.rs`：程序入口，只负责调用 `app::run()`。
- `app.rs`：组装状态、通道、播放线程、UI。
- `models`：数据结构，比如歌曲和曲库。
- `state`：应用运行时状态。
- `commands.rs`：UI 发给播放线程的命令。
- `events.rs`：播放线程发回 UI 的事件。
- `playback`：音频播放引擎，封装 `rodio`。
- `ui`：终端界面，封装 `crossterm`。
- `handlers`：把用户操作转成命令，把播放事件转成状态更新。

涉及的 Rust 知识点：

- `mod`：声明模块。
- `pub mod`：把模块暴露给外部使用。
- `crate::xxx`：从当前 crate 根路径引用模块。
- 文件夹模块：`models/mod.rs` 管理 `models/song.rs` 和 `models/library.rs`。

常见错误：

- 在 `main.rs` 里写太多业务逻辑。
- 每个函数都 `pub`，导致模块边界不清楚。
- 不知道该 `use crate::models::Song` 还是 `use super::Song`，初学时优先使用 `crate::...` 更直观。

## 第二步：设计数据模型

目标：先定义“数据长什么样”，不要在模型里写播放逻辑。

`models/song.rs`：

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Song {
    pub id: usize,
    pub title: String,
    pub path: PathBuf,
}
```

`models/library.rs`：

```rust
use crate::models::song::Song;

#[derive(Debug, Default, Clone)]
pub struct Library {
    pub songs: Vec<Song>,
}

impl Library {
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn get(&self, index: usize) -> Option<&Song>;
    pub fn next_index(&self, current: usize) -> Option<usize>;
    pub fn previous_index(&self, current: usize) -> Option<usize>;
}
```

为什么这样设计：

- `Song` 只描述一首歌，不负责播放。
- `Library` 只管理歌曲集合，不负责 UI。
- `PathBuf` 比 `String` 更适合表示文件路径。

优点：

- 后续 TUI 和 Slint GUI 都可以复用 `Song`、`Library`。
- 测试数据模型很简单，不需要真的播放音乐。

常见错误：

- 在 `Song` 里写 `play()` 方法，让模型依赖 `rodio`。
- 用 `String` 到处传路径，后面路径拼接和判断文件存在会不舒服。
- `id` 和 `Vec` 下标混用。`id` 是歌曲编号，`index` 是列表位置，两个概念不要混淆。

## 第三步：设计 PlaybackCommand

目标：定义 UI 可以向播放线程发送哪些“命令”。

`commands.rs`：

```rust
#[derive(Debug, Clone)]
pub enum PlaybackCommand {
    Play(usize),
    Pause,
    Resume,
    TogglePause,
    Stop,
    Next,
    Previous,
    Shutdown,
}
```

每个命令的含义：

- `Play(usize)`：播放指定下标的歌曲。
- `Pause`：暂停当前歌曲。
- `Resume`：继续播放。
- `TogglePause`：如果正在播放就暂停，如果暂停就继续。
- `Stop`：停止当前播放。
- `Next`：下一首。
- `Previous`：上一首。
- `Shutdown`：程序退出时关闭播放线程。

为什么用 enum：

- 播放命令是有限集合，很适合 Rust 的 `enum`。
- `match command` 可以强制你处理每一种情况。
- 比使用字符串命令更安全。

用到的方法：

```rust
use std::sync::mpsc::Sender;

pub fn send_play_command(
    tx: &Sender<PlaybackCommand>,
    command: PlaybackCommand,
) -> Result<(), String>;
```

常见错误：

- UI 直接调用播放函数，而不是发送命令。
- 用 `"pause"`、`"next"` 这种字符串表示命令，编译器帮不上忙。
- 忘记设计 `Shutdown`，导致播放线程不好退出。

## 第四步：设计 PlaybackEvent

目标：定义播放线程可以告诉 UI 什么事情发生了。

`events.rs`：

```rust
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    Started(usize),
    Paused,
    Resumed,
    Stopped,
    Finished(usize),
    SwitchedTo(usize),
    Error(String),
}
```

每个事件的含义：

- `Started(usize)`：某首歌开始播放。
- `Paused`：播放已暂停。
- `Resumed`：播放已继续。
- `Stopped`：播放已停止。
- `Finished(usize)`：某首歌自然播放结束。
- `SwitchedTo(usize)`：自动切换或手动切换到了另一首。
- `Error(String)`：播放失败，比如文件不存在或解码失败。

为什么需要事件：

- 播放线程不能直接改 UI。
- UI 线程收到事件后，再决定怎么更新 `AppState` 和界面。
- 这让播放层和 UI 层解耦。

常见错误：

- 在播放线程里直接 `println!` 或直接操作 UI。
- 只发命令不发事件，导致 UI 不知道播放状态变了。
- 错误信息直接 `panic!`，用户体验会很差。

## 第五步：实现 PlaybackState 和 AppState

目标：把“当前播放状态”集中管理。

`state/playback_state.rs`：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}
```

`state/app_state.rs`：

```rust
use crate::models::library::Library;
use crate::state::playback_state::PlaybackState;

#[derive(Debug)]
pub struct AppState {
    pub library: Library,
    pub current_index: Option<usize>,
    pub playback_state: PlaybackState,
    pub status_message: String,
}

impl AppState {
    pub fn new(library: Library) -> Self;
    pub fn current_song_title(&self) -> String;
    pub fn set_current_index(&mut self, index: usize);
    pub fn set_playback_state(&mut self, state: PlaybackState);
    pub fn set_status_message(&mut self, message: impl Into<String>);
}
```

为什么 `current_index` 用 `Option<usize>`：

- `Some(index)` 表示当前选中或正在播放某首歌。
- `None` 表示还没有当前歌曲。
- 比用 `-1` 更符合 Rust 习惯，因为 `usize` 本来就不能表示负数。

常见错误：

- 用多个布尔值表示状态，比如 `is_playing`、`is_paused`、`is_stopped`，容易互相矛盾。
- 长时间持有 `MutexGuard`，比如锁住状态后又去播放音乐。
- 在状态里保存太多底层对象，导致职责混乱。

## 第六步：实现播放线程

目标：让播放逻辑在独立线程运行，避免阻塞 UI。

`playback/engine.rs`：

```rust
use std::sync::mpsc::{Receiver, Sender};
use crate::commands::PlaybackCommand;
use crate::events::PlaybackEvent;
use crate::models::library::Library;

pub struct PlaybackEngine {
    library: Library,
    command_rx: Receiver<PlaybackCommand>,
    event_tx: Sender<PlaybackEvent>,
}

impl PlaybackEngine {
    pub fn new(
        library: Library,
        command_rx: Receiver<PlaybackCommand>,
        event_tx: Sender<PlaybackEvent>,
    ) -> Self;

    pub fn run(&mut self);
}

pub fn spawn_playback_thread(
    library: Library,
    command_rx: Receiver<PlaybackCommand>,
    event_tx: Sender<PlaybackEvent>,
) -> std::thread::JoinHandle<()>;
```

`playback/rodio_player.rs`：

```rust
use std::path::Path;

pub struct RodioPlayer {
    // 这里后面放 rodio 的播放器对象
}

impl RodioPlayer {
    pub fn new() -> Result<Self, String>;
    pub fn play_file(&mut self, path: &Path) -> Result<(), String>;
    pub fn pause(&self);
    pub fn resume(&self);
    pub fn stop(&mut self);
    pub fn is_empty(&self) -> bool;
}
```

播放线程大概做什么：

```rust
loop {
    match command_rx.recv() {
        Ok(PlaybackCommand::Play(index)) => {}
        Ok(PlaybackCommand::Pause) => {}
        Ok(PlaybackCommand::Resume) => {}
        Ok(PlaybackCommand::Next) => {}
        Ok(PlaybackCommand::Previous) => {}
        Ok(PlaybackCommand::Shutdown) | Err(_) => break,
        _ => {}
    }
}
```

为什么播放线程拥有 `RodioPlayer`：

- `rodio` 底层对象和音频设备关系紧密。
- 让播放线程独占播放器对象，线程安全问题更少。
- UI 不需要知道 `rodio` 怎么用。

常见错误：

- 在 UI 线程调用 `sleep_until_end()`。
- 每播放一首歌都重新设计一堆全局变量。
- 播放线程里不处理 `Err(_)`，发送端关闭后线程还不知道怎么退出。

## 第七步：实现 UI 与播放线程通信

目标：让键盘输入只产生命令，播放结果只通过事件回来。

`app.rs`：

```rust
use std::sync::{Arc, Mutex, mpsc};
use crate::commands::PlaybackCommand;
use crate::events::PlaybackEvent;
use crate::state::app_state::AppState;

pub fn run() -> Result<(), String> {
    let state = Arc::new(Mutex::new(AppState::new(load_library()?)));

    let (command_tx, command_rx) = mpsc::channel::<PlaybackCommand>();
    let (event_tx, event_rx) = mpsc::channel::<PlaybackEvent>();

    // 1. 启动播放线程
    // 2. 启动 TUI 主循环
    // 3. UI 发送 command_tx
    // 4. UI 读取 event_rx 并更新 state

    Ok(())
}
```

`ui/terminal.rs`：

```rust
use std::sync::{Arc, Mutex, mpsc::{Receiver, Sender}};
use crate::commands::PlaybackCommand;
use crate::events::PlaybackEvent;
use crate::state::app_state::AppState;

pub fn run_terminal_ui(
    state: Arc<Mutex<AppState>>,
    command_tx: Sender<PlaybackCommand>,
    event_rx: Receiver<PlaybackEvent>,
) -> Result<(), String>;
```

`handlers/playback_handler.rs`：

```rust
use std::sync::mpsc::Sender;
use crate::commands::PlaybackCommand;

pub fn play_selected(tx: &Sender<PlaybackCommand>, index: usize) -> Result<(), String>;
pub fn toggle_pause(tx: &Sender<PlaybackCommand>) -> Result<(), String>;
pub fn next(tx: &Sender<PlaybackCommand>) -> Result<(), String>;
pub fn previous(tx: &Sender<PlaybackCommand>) -> Result<(), String>;
```

`handlers/event_handler.rs`：

```rust
use crate::events::PlaybackEvent;
use crate::state::app_state::AppState;

pub fn apply_playback_event(
    state: &mut AppState,
    event: PlaybackEvent,
);
```

涉及的 Rust 知识点：

- `mpsc::channel`：多生产者、单消费者通道。
- `Sender<T>`：发送消息。
- `Receiver<T>`：接收消息。
- `try_recv()`：非阻塞接收事件，适合 UI 循环。
- `recv()`：阻塞等待命令，适合播放线程。

常见错误：

- UI 循环里用 `recv()` 等事件，会卡住键盘输入。
- 播放线程里用 `try_recv()` 忙等，不加休眠会浪费 CPU。
- 一边拿着 `Mutex` 锁，一边做耗时操作。

## 第八步：实现自动切歌

目标：当前歌曲播放完成后，播放线程自动切到下一首，并通知 UI。

核心思路：

- 播放线程知道当前播放的 `current_index`。
- 播放线程能检查当前音频是否结束。
- 如果结束，根据播放模式计算下一首。
- 播放下一首，并发送 `PlaybackEvent::Finished` 和 `PlaybackEvent::SwitchedTo`。

预留播放模式：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayMode {
    Sequential,
    RepeatOne,
    Shuffle,
}
```

下一首计算函数：

```rust
use crate::models::library::Library;

pub fn next_index(
    library: &Library,
    current_index: usize,
    mode: PlayMode,
) -> Option<usize>;
```

自动切歌时播放线程大概做什么：

```rust
if player.is_empty() {
    event_tx.send(PlaybackEvent::Finished(current_index)).ok();

    if let Some(next) = next_index(&library, current_index, play_mode) {
        current_index = next;
        player.play_file(&library.songs[next].path)?;
        event_tx.send(PlaybackEvent::SwitchedTo(next)).ok();
    } else {
        event_tx.send(PlaybackEvent::Stopped).ok();
    }
}
```

为什么自动切歌放在播放线程：

- 播放线程最清楚音频有没有结束。
- UI 不应该轮询 rodio 内部状态。
- 后续 Slint GUI 也可以复用这套播放逻辑。

常见错误：

- 播放结束后让 UI 再决定播放下一首，导致播放逻辑分散。
- 播放线程直接修改 UI 状态，后续迁移 Slint 会更乱。
- 忘记处理最后一首歌，导致数组越界。

## 推荐实现顺序

我们后面实际写代码时，按这个顺序来：

1. 创建目录和空模块，让 `cargo check` 通过。
2. 把 `Song`、`Library` 从旧 `lib.rs` 拆到 `models`。
3. 新建 `PlaybackCommand`，先只定义 enum，不接播放。
4. 新建 `PlaybackEvent`，先只定义 enum。
5. 新建 `PlaybackState` 和 `AppState`。
6. 建立 `mpsc` 通道，先发送假命令测试通信。
7. 写播放线程骨架，先能启动和退出。
8. 把 `rodio` 播放单首歌接入播放线程。
9. 加暂停、继续、上一首、下一首。
10. 加自动切歌。

## 2026-06-13 当前第一阶段 TUI 已补齐

本轮已经把第一阶段 TUI 的主干补齐，重点不是界面华丽，而是让架构真正跑起来：

- `src/main.rs`：恢复为真正入口，默认 `cargo run` 启动 TUI，`list/scan/search` 继续可用。
- `src/lib.rs`：声明新架构模块，并保留曲库读取、保存、扫描、搜索等基础函数。
- `src/models`：放 `Song` 和 `Library`，并提供 `next_index`、`previous_index`。
- `src/state`：放 `AppState`、`PlaybackState` 和 `SharedAppState = Arc<Mutex<AppState>>`。
- `src/commands.rs`：定义 UI 发给播放线程的 `PlaybackCommand`。
- `src/events.rs`：定义播放线程发回 UI 的 `PlaybackEvent`。
- `src/playback`：播放线程和 `rodio` 封装，支持播放、暂停、继续、上一首、下一首、停止、自动切歌。
- `src/ui_terminal`：终端界面，支持方向键选择、Enter 播放、空格暂停/继续、`n/p` 上下一首、`s` 停止、`q` 退出。
- `src/handlers`：把用户操作转成命令，把播放事件转成状态更新。

第一阶段还没有做的内容：

- 没有显示真实播放进度。
- 没有显示歌曲总时长。
- 没有 Seek 拖动或跳转。
- 没有音量控制。
- 没有随机播放和单曲循环。
- 没有把 TUI 状态同步到 Slint GUI。

## 当前 TUI 使用方式

先扫描音乐文件夹：

```powershell
cargo run -- scan E:\Music
```

启动 TUI 播放器：

```powershell
cargo run
```

也可以显式启动：

```powershell
cargo run -- play
```

TUI 按键：

- `Up/Down`：选择歌曲。
- `Enter`：播放当前选中歌曲。
- `Space`：暂停或继续。
- `n` 或右方向键：下一首。
- `p` 或左方向键：上一首。
- `s`：停止播放。
- `q` 或 `Esc`：退出。

## 下一步学习建议

下一步不要马上接 Slint。建议先把 TUI 的线程模型吃透，因为 GUI 也会复用同样的思想。

建议阅读顺序：

1. `src/main.rs`：看程序如何从 CLI 进入 `app::run()`。
2. `src/app.rs`：看 `Arc<Mutex<AppState>>`、`mpsc::channel`、播放线程、TUI 是怎么组装起来的。
3. `src/commands.rs` 和 `src/events.rs`：理解 UI 和播放线程的通信协议。
4. `src/playback/engine.rs`：理解播放线程如何处理命令和自动切歌。
5. `src/ui_terminal/terminal.rs`：理解 UI 主循环为什么用 `try_recv()` 接事件，而不是阻塞等待。
6. `src/handlers/event_handler.rs`：理解事件如何变成状态更新。

下一次适合做的练习：

- 给 `PlaybackEngine` 增加 `PlayMode`：顺序播放、单曲循环、列表循环。
- 给 TUI 增加音量快捷键：`+` 增加音量，`-` 降低音量。
- 给 `RodioPlayer` 暴露 `position()`，在 TUI 显示当前播放秒数。
- 把 `library.json` 的乱码问题排查清楚，确保中文歌名保存和读取都是 UTF-8。

## 对新手最重要的理解

这套架构不是为了显得复杂，而是为了让每个问题都只出现在一个地方：

- 歌曲数据错了，看 `models`。
- 当前状态错了，看 `state`。
- 按键没反应，看 `ui/input` 或 `handlers`。
- 播放失败，看 `playback/rodio_player.rs`。
- 自动切歌错了，看 `playback/engine.rs`。

你现在害怕 Rust 很正常，因为 Rust 会把“所有权、生命周期、线程安全”这些东西提前暴露出来。我们的做法是把项目拆小，让每次只面对一个概念。先让模块边界清楚，再一点点把功能接上，Rust 就不会像一整面墙压过来。
