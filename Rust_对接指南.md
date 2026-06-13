# Rust 侧对接全指南 · 从 Slint UI 到播放逻辑

> **目标**：让你对照着 SLint 的 `app-window.slint`，写出完整可用的 Rust 代码。  
> **本文替你解决**：每个 callback 绑什么函数、property 怎么读写、线程在哪里开、`Arc<Mutex<T>>` 什么时候用。

---

## 目录

1. [架构总览与数据流](#1-架构总览与数据流)
2. [类型映射表（Slint ↔ Rust）](#2-类型映射表slint--rust)
3. [Slint 生成的 Rust API 签名全表](#3-slint-生成的-rust-api-签名全表)
4. [14 个 callback 的业务逻辑详解](#4-14-个-callback-的业务逻辑详解)
5. [多线程架构](#5-多线程架构)
6. [Arc<Mutex<T>> 使用场景](#6-arcmutext-使用场景)
7. [关键 Slint Rust API 速查](#7-关键-slint-rust-api-速查)
8. [main.rs 骨架](#8-mainrs-骨架)
9. [Cargo.toml 新增依赖](#9-cargotoml-新增依赖)

---

## 1. 架构总览与数据流

```
┌──────────────────────────────────────────────────────┐
│  main.rs                                             │
│  ┌──────────────────────────────────────────────────┐│
│  │ AppWindow::new()                                 ││
│  │   ↓ 填充导航 / 加载歌库							 ││
│  │   ↓ 绑定全部 callback								|│
│  │   ↓ 启动 timer                                    ││
│  │   → app.run()     ← 事件循环（阻塞主线程）           ││
│  └──────────────────────────────────────────────────┘│
│          ▲                  │                        │
│          │ set_xxx()        │ callback 触发           │
│          │                  ▼                        │
│  ┌──────────────────────────────────────────────────┐│
│  │ 回调处理函数                                       ││
│  │  ├─ 同步操作（search / volume / 切歌更新 UI）       ││
│  │  └─ 启动线程（play / scan）                        ││
│  └──────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────┘
          ▲ channel                          │ thread::spawn
          │                                  ▼
┌──────────────────────────────────────────────────────┐
│  后台播放线程（rodio）                                  │
│  → 播放 / 暂停 / 停止                                  │
│  → 播完发送 PlayerEvent::Finished                     │
└──────────────────────────────────────────────────────┘
```

### 关键原则

| 原则 | 说明 |
|------|------|
| **app.run() 阻塞主线程** | Slint 事件循环接管后不再返回，所有耗时操作必须在其他线程 |
| **UI 更新必须回主线程** | 后台线程调 `slint::invoke_from_event_loop(f \|\| app.set_xxx(...))` |
| **play_state + library 共享** | 用 `Arc<Mutex<...>>` 包裹 |
| **rodio 不换线程** | 当前 `DeviceSinkBuilder + Player` 模式可用，但 `sleep_until_end()` 阻塞 → 必须放在后台线程 |

---

## 2. 类型映射表（Slint ↔ Rust）

### Slint 基本类型 → Rust 类型

| Slint 类型 | Rust 类型 | 转换方法 |
|-----------|----------|---------|
| `int` | `i32` | 直接传 |
| `float` | `f32` | 直接传 |
| `bool` | `bool` | 直接传 |
| `string` | `slint::SharedString` | `"xxx".into()` 或 `.to_string()` 互转 |
| `image` | `slint::Image` | `slint::Image::load_from_path(&p)?` |

### Slint 自定义类型 → Rust 类型

这些是 `build.rs` 编译后自动生成的 Rust 类型（在 `slint::include_modules!()` 中）：

```rust
// 枚举：Slint 的 export enum PlayState { Stopped, Playing, Paused }
// ↓ 自动生成
#[derive(Clone, Debug, PartialEq)]
pub enum PlayState {
    Stopped,
    Playing,
    Paused,
}

// 结构体：Slint 的 export struct SongData { title, path, artist, duration }
// ↓ 自动生成
#[derive(Clone, Debug)]
pub struct SongData {
    pub title: slint::SharedString,
    pub path: slint::SharedString,
    pub artist: slint::SharedString,
    pub duration: slint::SharedString,
}

// 结构体：Slint 的 export struct NavItem { label }
// ↓ 自动生成
#[derive(Clone, Debug)]
pub struct NavItem {
    pub label: slint::SharedString,
}
```

> 你**不需要**手写这些类型！`slint_build::compile()` + `slint::include_modules!()` 自动生成。

### 数组类型

| Slint | Rust |
|-------|------|
| `[SongData]` | `slint::ModelRc<SongData>` |
| `[NavItem]` | `slint::ModelRc<NavItem>` |

构造方式：

```rust
use slint::{ModelRc, VecModel, SharedString};

let model: ModelRc<SongData> = VecModel::default().into();
// ↓ 添加元素
VecModel::from(model).push(SongData {
    title: "晴天".into(),
    path: "/music/晴天.mp3".into(),
    artist: "周杰伦".into(),
    duration: "04:29".into(),
});
app.set_songs(model.into());  // 注意：需要 .into() 到 ModelRc
```

---

## 3. Slint 生成的 Rust API 签名全表

> 以下所有方法由 `slint_build::compile("ui/app-window.slint")` 编译后，通过 `slint::include_modules!()` 自动生成在 `AppWindow` 类型上。

### 3.1 Property 的 set_ / get_ 方法

| Slint 声明 | Rust set_ 方法 | Rust get_ 方法 | 取值示例 |
|-----------|---------------|---------------|---------|
| `in property <[NavItem]> nav-items` | `set_nav_items(ModelRc<NavItem>)` | — | `VecModel` 构造 |
| `in-out property <int> current-nav-index: 0` | `set_current_nav_index(i32)` | `get_current_nav_index() -> i32` | 0, 1, 2 |
| `in property <[SongData]> songs` | `set_songs(ModelRc<SongData>)` | — | `VecModel` 构造 |
| `in-out property <int> current-song-index: -1` | `set_current_song_index(i32)` | `get_current_song_index() -> i32` | -1 表示未选 |
| `in-out property <string> current-song-title: "..."` | `set_current_song_title(SharedString)` | `get_current_song_title() -> SharedString` | `song.title.into()` |
| `in-out property <string> current-song-artist: "—"` | `set_current_song_artist(SharedString)` | `get_current_song_artist() -> SharedString` | |
| `in-out property <string> current-song-path: "—"` | `set_current_song_path(SharedString)` | `get_current_song_path() -> SharedString` | |
| `in-out property <string> duration-total: "00:00"` | `set_duration_total(SharedString)` | `get_duration_total() -> SharedString` | `"04:29".into()` |
| `in-out property <string> duration-current: "00:00"` | `set_duration_current(SharedString)` | `get_duration_current() -> SharedString` | |
| `in-out property <float> progress: 0.0` | `set_progress(f32)` | `get_progress() -> f32` | 0.0 ~ 100.0 |
| `in-out property <PlayState> play-state: Stopped` | `set_play_state(PlayState)` | `get_play_state() -> PlayState` | `PlayState::Playing` |
| `in-out property <string> now-playing-text: "—"` | `set_now_playing_text(SharedString)` | `get_now_playing_text() -> SharedString` | `"晴天".into()` |
| `in-out property <float> volume: 50.0` | `set_volume(f32)` | `get_volume() -> f32` | 0.0 ~ 100.0 |
| `in-out property <bool> shuffle-on: false` | `set_shuffle_on(bool)` | `get_shuffle_on() -> bool` | |
| `in-out property <bool> loop-on: false` | `set_loop_on(bool)` | `get_loop_on() -> bool` | |

### 3.2 Callback 的 on_ 绑定方法

| Slint 声明 | Rust 绑定签名 | 何时触发 |
|-----------|-------------|---------|
| `callback search-submitted(string)` | `on_search_submitted(impl Fn(SharedString) + 'static)` | 用户回车搜索 |
| `callback scan-folder-clicked()` | `on_scan_folder_clicked(impl Fn() + 'static)` | 点扫描图标 |
| `callback nav-item-selected(int)` | `on_nav_item_selected(impl Fn(i32) + 'static)` | 点导航项 |
| `callback song-selected(int)` | `on_song_selected(impl Fn(i32) + 'static)` | 单击歌曲行 |
| `callback song-double-clicked(int)` | `on_song_double_clicked(impl Fn(i32) + 'static)` | 双击歌曲行 |
| `callback seek-changed(float)` | `on_seek_changed(impl Fn(f32) + 'static)` | 拖动进度条 |
| `callback open-file-location(string)` | `on_open_file_location(impl Fn(SharedString) + 'static)` | 点"打开文件位置" |
| `callback prev-clicked()` | `on_prev_clicked(impl Fn() + 'static)` | 点上一首 |
| `callback play-pause-clicked()` | `on_play_pause_clicked(impl Fn() + 'static)` | 播放/暂停按钮 |
| `callback next-clicked()` | `on_next_clicked(impl Fn() + 'static)` | 点下一首 |
| `callback shuffle-toggled(bool)` | `on_shuffle_toggled(impl Fn(bool) + 'static)` | 切换随机 |
| `callback loop-toggled(bool)` | `on_loop_toggled(impl Fn(bool) + 'static)` | 切换循环 |
| `callback volume-changed(float)` | `on_volume_changed(impl Fn(f32) + 'static)` | 拖动音量滑块 |

### 3.3 命名规则速记

```rust
// Slint: in-out property <int> my-val-name: 0
// Rust:  app.set_my_val_name(42);
//         app.get_my_val_name() -> i32

// Slint: callback my-callback(int, string)
// Rust:  app.on_my_callback(|a: i32, b: SharedString| { ... });

// Slint: property — 中划线变下划线，驼峰
//        callback — 同上
// my-val-name → my_val_name
```

---

## 4. 14 个 callback 的业务逻辑详解

### 4.1 `search-submitted(keyword)` — 搜索

```rust
// 绑定
app.on_search_submitted({
    let app_weak = app.as_weak();
    let library = library.clone(); // Arc<Mutex<Library>>
    move |keyword: slint::SharedString| {
        let lib = library.lock().unwrap();
        let kw = keyword.to_lowercase();

        // 1. 过滤
        let filtered: Vec<&Song> = lib.songs.iter()
            .filter(|s| s.title.to_lowercase().contains(&kw))
            .collect();

        // 2. 构建新的 VecModel
        let model: slint::VecModel<SongData> = slint::VecModel::default();
        for song in &filtered {
            model.push(SongData {
                title: song.title.clone().into(),
                path: song.path.clone().into(),
                artist: "".into(),       // 你的 Song 没有 artist 字段，先留空
                duration: "".into(),     // 同上
            });
        }

        // 3. 更新 UI（注意 set_ 必须在主线程）
        let app = app_weak.upgrade().unwrap();
        app.set_songs(model.into());
    }
});
```

### 4.2 `scan-folder-clicked()` — 扫描文件夹

```rust
// 推荐方案：打开系统文件对话框
// 需要添加 native-dialog crate
app.on_scan_folder_clicked({
    let app_weak = app.as_weak();
    let library = library.clone();
    move || {
        // 1. 弹对话框
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            // 2. 后台线程扫描（避免 UI 卡）
            let lib = library.clone();
            let app_w = app_weak.clone();
            std::thread::spawn(move || {
                let mut lib = lib.lock().unwrap();
                let _ = scan_folder(path.to_str().unwrap_or(""), &mut lib);
                let _ = save_library(MUSIC_FILE_PATH, &lib);

                // 3. 扫描后刷新列表
                let _ = slint::invoke_from_event_loop(move || {
                    let app = app_w.upgrade().unwrap();
                    refresh_song_list(&app, &lib);
                });
            });
        }
    }
});
```

### 4.3 `nav-item-selected(idx)` — 切换导航

```rust
app.on_nav_item_selected(move |idx: i32| {
    match idx {
        0 => println!("全部歌曲"),  // 显示全部
        1 => println!("最近播放"),  // 按播放历史排序
        2 => println!("收藏"),      // 只显示收藏歌曲
        _ => {}
    }
    // 预留：未来修改 songs 列表过滤条件
});
```

### 4.4 `song-selected(idx)` — 单击选中歌曲（更新详情）

```rust
app.on_song_selected({
    let app_weak = app.as_weak();
    let library = library.clone();
    move |idx: i32| {
        let lib = library.lock().unwrap();
        if let Some(song) = lib.songs.get(idx as usize) {
            let app = app_weak.upgrade().unwrap();
            app.set_current_song_title(song.title.clone().into());
            app.set_current_song_path(song.path.clone().into());
            app.set_detail_visible(true);
            // artist / duration 暂无，留空
        }
    }
});
```

### 4.5 `song-double-clicked(idx)` — 双击直接播放

```rust
app.on_song_double_clicked({
    let app_weak = app.as_weak();
    let library = library.clone();
    let player_state = player_state.clone(); // Arc<Mutex<PlayerState>>
    let tx = tx.clone(); // mpsc::Sender<PlayerEvent>
    move |idx: i32| {
        // 1. 更新选中
        app_weak.upgrade().unwrap().set_current_song_index(idx);

        // 2. 触发播放
        start_playing(&library, &player_state, &tx, idx as usize);
    }
});
```

### 4.6 `play-pause-clicked()` — 播放/暂停

```rust
app.on_play_pause_clicked({
    let app_weak = app.as_weak();
    let play_state = play_state.clone();
    move || {
        let mut ps = play_state.lock().unwrap();
        match *ps {
            PlaybackState::Stopped => {
                // 没有选中歌曲时不动作
            }
            PlaybackState::Playing { .. } => {
                // → 暂停
                if let Some(sink) = &ps.sink {
                    sink.pause();
                }
                *ps = PlaybackState::Paused { song_idx: ps.current_song_index().unwrap_or(0) };
                // 更新 UI
                let app = app_weak.upgrade().unwrap();
                app.set_play_state(PlayState::Paused);
            }
            PlaybackState::Paused { .. } => {
                // → 恢复播放
                if let Some(sink) = &ps.sink {
                    sink.play();
                }
                *ps = PlaybackState::Playing { song_idx: ps.current_song_index().unwrap_or(0) };
                let app = app_weak.upgrade().unwrap();
                app.set_play_state(PlayState::Playing);
            }
        }
    }
});
```

### 4.7 `prev-clicked()` / `next-clicked()` — 切歌

```rust
app.on_prev_clicked({
    let app_weak = app.as_weak();
    let library = library.clone();
    let play_state = play_state.clone();
    let tx = tx.clone();
    move || {
        let ps = play_state.lock().unwrap();
        let total = library.lock().unwrap().songs.len();
        if total == 0 { return; }
        let current = ps.current_song_index().unwrap_or(0);
        drop(ps);
        let prev = (current + total - 1) % total;
        app_weak.upgrade().unwrap().set_current_song_index(prev as i32);
        start_playing(&library, &play_state, &tx, prev);
    }
});

// next-clicked 同理，prev = (current + 1) % total
```

### 4.8 `seek-changed(percent)` — 拖动进度条

```rust
app.on_seek_changed(move |percent: f32| {
    let ps = play_state.lock().unwrap();
    if let Some(sink) = &ps.sink {
        let total = /* 从 sink 或缓存的总时长 */;
        let target = Duration::from_secs_f32(total.as_secs_f32() * percent / 100.0);
        sink.try_seek(target).ok();
    }
});
```

### 4.9 `volume-changed(vol)` — 音量调节

```rust
app.on_volume_changed(move |vol: f32| {
    let ps = play_state.lock().unwrap();
    if let Some(sink) = &ps.sink {
        sink.set_volume(vol / 100.0); // rodio 的 volume 范围 0.0~1.0
    }
});
```

### 4.10 `open-file-location(path)` — 打开文件管理器

```rust
app.on_open_file_location(move |path: slint::SharedString| {
    // 使用 open crate
    let _ = open::that(&*path.to_string());
});
```

需要加依赖：`open = "5"`

### 4.11 `shuffle-toggled(on)` / `loop-toggled(on)` — 播放模式

```rust
app.on_shuffle_toggled(move |on: bool| {
    println!("随机播放: {}", on);
    // 预留：切歌时随机选下一首
});

app.on_loop_toggled(move |on: bool| {
    println!("循环播放: {}", on);
    // 预留：最后一首播完回到第一首
});
```

---

## 5. 多线程架构

### 5.1 为什么要多线程

| 原因 | 具体 |
|------|------|
| `app.run()` 阻塞主线程 | 所有 UI 事件处理都在主线程 |
| `sleep_until_end()` 阻塞 | rodio 播放时阻塞当前线程 |
| UI 不能卡 | 如果在主线程播放，窗口会卡死，无法切歌 |

### 5.2 线程布局

```
┌─── 主线程 (Slint 事件循环) ────────────────────────┐
│  app.run()                                       │
│  ├── 处理 UI 事件（按钮、搜索、列表点击）              │
│  ├── Timer 每 200ms 运行：                         │
│  │    ├── 尝试 rx.try_recv() 接收播放事件          │
│  │    ├── 读取 rodio sink 进度 → set_progress()   │
│  │    └── 更新 duration-current                  │
│  └── invoke_from_event_loop 投递的 UI 更新        │
└──────────────────────────────────────────────────┘
                        ▲
              tx.send() │ rx.try_recv()
                        │
┌─── 播放线程 (thread::spawn) ─────────────────────┐
│  rodio Player: 解码 → 播放 → sleep_until_end()   │
│  播完 → tx.send(PlayerEvent::SongFinished)        │
│  如果需要 → 定时发 PlayerEvent::Tick(pos)         │
└──────────────────────────────────────────────────┘
```

### 5.3 消息类型枚举

```rust
// 播放线程 → 主线程的通信消息
enum PlayerEvent {
    /// 当前歌曲播放完毕
    Finished,
    /// 播放进度（秒）
    Tick(f32),
    /// 播放出错
    Error(String),
}
```

### 5.4 通道的创建

```rust
use std::sync::mpsc;

// 创建通道
let (tx, rx) = mpsc::channel::<PlayerEvent>();

// tx: 克隆给播放线程
// rx: 由主线程的 Timer 轮询 (try_recv)
```

### 5.5 播放线程的启动

```rust
fn start_playing(
    library: &Arc<Mutex<Library>>,
    play_state: &Arc<Mutex<PlaybackState>>,
    tx: &mpsc::Sender<PlayerEvent>,
    song_idx: usize,
) {
    // 1. 读取歌曲路径
    let path = library.lock().unwrap()
        .songs.get(song_idx)
        .map(|s| s.path.clone());

    let path = match path {
        Some(p) => p,
        None => return,
    };

    // 2. 更新状态
    let mut ps = play_state.lock().unwrap();
    *ps = PlaybackState::Playing {
        song_idx,
        sink: None, // 由播放线程创建
    };
    drop(ps); // 尽快释放锁

    // 3. 启动播放线程
    let tx = tx.clone();
    std::thread::spawn(move || {
        match play_audio_file(&path) {
            Ok(()) => {
                tx.send(PlayerEvent::Finished).ok();
            }
            Err(e) => {
                tx.send(PlayerEvent::Error(e.to_string())).ok();
            }
        }
    });
}

// 纯播放函数（在新线程中执行）
fn play_audio_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, handle) = rodio::OutputStream::try_default()?;
    let sink = rodio::Sink::try_new(&handle)?;

    let file = std::fs::File::open(path)?;
    let source = rodio::Decoder::new(file)?;

    sink.append(source);
    sink.sleep_until_end(); // 阻塞直到播完
    Ok(())
}
```

### 5.6 Timer 轮询消息 + 更新进度

```rust
use slint::{Timer, TimerMode};

let app_weak = app.as_weak();
let rx = Arc::new(Mutex::new(rx));
// 注：PlaybackState 里需要存 sink 的句柄，才能读取进度
// rodio 的 Sink 提供 len()（总秒数）和 get_pos()（已播秒数）

let timer = Timer::default();
timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), {
    let app_weak = app_weak.clone();
    let rx = rx.clone();
    let play_state = play_state.clone();
    move || {
        // ① 处理通道消息
        let rx_lock = rx.lock().unwrap();
        loop {
            match rx_lock.try_recv() {
                Ok(PlayerEvent::Finished) => {
                    let app = app_weak.upgrade().unwrap();
                    app.set_play_state(PlayState::Stopped);
                    // 自动下一首（根据 loop-on / shuffle-on）
                    handle_song_finished(&app, &play_state, &library, &tx);
                }
                Ok(PlayerEvent::Tick(pos)) => {
                    let app = app_weak.upgrade().unwrap();
                    app.set_duration_current(format_duration(pos).into());
                }
                Ok(PlayerEvent::Error(msg)) => {
                    eprintln!("播放错误: {}", msg);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
        drop(rx_lock);

        // ② 更新进度（从 rodio Sink 读取）
        let ps = play_state.lock().unwrap();
        if let Some(sink) = &ps.sink {
            let pos = sink.get_pos().as_secs_f32();
            let total = sink.len().as_secs_f32();
            if total > 0.0 {
                let app = app_weak.upgrade().unwrap();
                app.set_progress(pos / total * 100.0);
            }
        }
    }
});
```

---

## 6. Arc<Mutex<T>> 使用场景

### 6.1 哪些数据需要共享

| 数据 | 类型 | 被谁读 | 被谁写 | 需要 Arc<Mutex>? |
|------|------|--------|--------|-----------------|
| `Library`（歌库） | `Library` 结构体 | UI 回调、播放线程启动 | scan 回调 | ✅ **需要** |
| `PlayerState`（播放状态） | 自定义 `PlaybackState` struct | Timer（读进度）、暂停/切歌 callback | 播放线程（写 finish） | ✅ **需要** |
| `mpsc::Receiver` | 标准库 | Timer 轮询 | — | ✅ **需要**（只能一个线程持有） |
| `VecModel<SongData>` | 在 Slint 内部 | Slint 自己管理 | callback 重建新 model | ❌ 不需要 |

### 6.2 PlaybackState 结构体设计

```rust
/// 播放器运行时状态（被 Arc<Mutex<>> 包裹）
pub struct PlaybackState {
    pub current_song_idx: usize,
    pub status: PlayStatus,
    /// rodio Sink——可暂停/恢复/读进度
    pub sink: Option<rodio::Sink>,
    pub stream_handle: Option<rodio::OutputStreamHandle>,
    pub _stream: Option<rodio::OutputStream>,
}

pub enum PlayStatus {
    Stopped,
    Playing,
    Paused,
}
```

### 6.3 初始化共享状态

```rust
// 在 main() 中创建
let library = Arc::new(Mutex::new(load_library("library.json").unwrap()));
let play_state = Arc::new(Mutex::new(PlaybackState {
    current_song_idx: 0,
    status: PlayStatus::Stopped,
    sink: None,
    stream_handle: None,
    _stream: None,
}));
let (tx, rx) = mpsc::channel::<PlayerEvent>();
let rx = Arc::new(Mutex::new(rx));
```

### 6.4 克隆规则

```rust
// 每次克隆只增加引用计数，不是深拷贝
let lib_a = library.clone();  // 给 scan callback
let lib_b = library.clone();  // 给 song-selected callback
let lib_c = library.clone();  // 给播放线程
```

---

## 7. 关键 Slint Rust API 速查

### 7.1 `slint::include_modules!()` 宏

```rust
// build.rs 里定义编译入口：
//   slint_build::compile("ui/app-window.slint").unwrap();
//
// main.rs 里引入生成的代码：
slint::include_modules!();
// 这会生成 AppWindow 等类型
```

> 必须放在 `main.rs` 中，不能在 `lib.rs` 里。

### 7.2 `app.as_weak()` 弱引用

```rust
// 在闭包中持有 app 的强引用会导致循环引用：
// app → on_xxx(闭包) → app（无法释放）
// ❌ 不要这样：
app.on_clicked(move || {
    app.set_xxx();  // app 被闭包持有 → 循环引用
});

// ✅ 正确做法：
let app_weak = app.as_weak();
app.on_clicked(move || {
    let app = app_weak.upgrade().unwrap(); // 升级为强引用
    app.set_xxx();
    // 闭包结束时 app 自动释放
});
```

### 7.3 `slint::invoke_from_event_loop()`

```rust
/// 从后台线程安全地更新 UI
/// 签名:
pub fn invoke_from_event_loop<F>(func: F) -> Result<(), EventLoopError>
where
    F: FnOnce() + Send + 'static,

// 用法:
let app_weak = app.as_weak();  // 必须在主线程获取
std::thread::spawn(move || {
    // ... 耗时操作 ...
    let _ = slint::invoke_from_event_loop(move || {
        let app = app_weak.upgrade().unwrap();
        app.set_progress(100.0);
    });
});
```

### 7.4 `slint::Timer`

```rust
/// 周期性或单次定时器
/// 构造与模式:
let timer = slint::Timer::default();

timer.start(
    slint::TimerMode::Repeated,  // Repeated 重复 / SingleShot 只触发一次
    std::time::Duration::from_millis(200),
    move || {
        // 在这里可以直接调 app.set_xxx()（不需要 invoke_from_event_loop）
        // 因为 Timer 本身就在事件循环所在的线程运行
        let app = app_weak.upgrade().unwrap();
        app.set_progress(new_val);
    },
);

// 停止定时器
timer.stop();
```

### 7.5 `VecModel` / `ModelRc`

```rust
use slint::{VecModel, ModelRc, Model};

// 创建空模型
let model: ModelRc<SongData> = VecModel::default().into();

// 通过 ModelRc 无法直接 push，需要得回 VecModel 再放回去
// 安全的做法：
let vec_model = VecModel::from(model.clone());
vec_model.push(SongData { title: "歌名".into(), path: "/a.mp3".into(), artist: "".into(), duration: "03:00".into() });

// 替换 UI 的所有数据
app.set_songs(model.into());

// 清空
let new_model: ModelRc<SongData> = VecModel::default().into();
app.set_songs(new_model.into());
```

### 7.6 `SharedString` ↔ `String` 转换

```rust
// String → SharedString (推荐 using .into())
let shared: slint::SharedString = "晴天".into();
let shared: slint::SharedString = my_string.clone().into();

// SharedString → String
let s: String = shared.to_string();
let s: &str = shared.as_str();
```

### 7.7 `slint::Image` 加载

```rust
// 从文件路径加载
let image = slint::Image::load_from_path("covers/album.jpg").ok();
app.set_cover_image(image.unwrap_or_default());

// 从内存加载
let data = std::fs::read("cover.png").unwrap();
let image = slint::Image::load_from_bytes(&data).ok();
```

### 7.8 文件对话框

```rust
// 方案一：使用 rfd crate（需要添加到 Cargo.toml）
let folder = rfd::FileDialog::new()
    .set_title("选择音乐文件夹")
    .pick_folder();

// 方案二：使用 tinyfiledialogs
```

---

## 8. main.rs 骨架（重构版）

> **先说问题**：旧的 `main()` 函数 230 行，所有 callback 闭包内联、重复 clone、状态分散在松散变量中——典型的 God main 反模式。  
> **重构目标**：把所有共享状态打包进 `AppState` 结构体；每个 callback 绑定到一个独立方法；`main()` 精简到 20 行作为纯编排者。

### 8.1 新模块结构

```
src/
├── main.rs          ← 🧭 编排者：new → configure → run
├── lib.rs           ← 已有：Library / Song / scan / save / is_music_file（不动）
└── player/
    ├── mod.rs       ← 重新导出
    ├── state.rs     ← AppState 结构体（所有 Arc<Mutex<>> 统一管理）
    ├── handlers.rs  ← 14 个 callback 的 on_xxx 方法
    └── playback.rs  ← 播放线程 + channel 消息类型 + start_playing
```

### 8.2 `main.rs` — 只有 20 行

```rust
// src/main.rs
slint::include_modules!();

mod player;
use player::state::AppState;

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let state = AppState::new(&app);
    state.register_callbacks(&app);
    let _timer = state.start_timer(&app);
    app.run()
}
```

`main.rs` 做的事情只有一个：`new → register → timer → run`。所有的"怎么做"都委托给了 `AppState`。

---

### 8.3 `state.rs` — AppState 统一管理所有共享状态

```rust
// src/player/state.rs
use std::sync::{Arc, Mutex, mpsc};
use slint::{Timer, TimerMode, VecModel, ModelRc};
use crate::player::playback::{self, PlaybackState, PlayerEvent, PlayStatus};

/// 应用全局状态 —— 所有 Arc<Mutex<>> 收拢在此
pub struct AppState {
    pub library: Arc<Mutex<crate::Library>>,
    pub play_state: Arc<Mutex<PlaybackState>>,
    /// 后台线程发过来给 Timer 轮询的通道接收端
    pub rx: Arc<Mutex<mpsc::Receiver<PlayerEvent>>>,
    /// 播放线程发消息用的发送端
    pub tx: mpsc::Sender<PlayerEvent>,
}

impl AppState {
    /// 构造共享状态 + 初始加载歌库
    pub fn new(app: &crate::AppWindow) -> Self {
        let library = Arc::new(Mutex::new(
            crate::load_library(crate::MUSIC_FILE_PATH).unwrap_or_default()
        ));

        let play_state = Arc::new(Mutex::new(PlaybackState::default()));
        let (tx, rx) = mpsc::channel::<PlayerEvent>();

        let state = Self {
            library,
            play_state,
            rx: Arc::new(Mutex::new(rx)),
            tx,
        };

        // 初始加载歌单到 UI
        Self::refresh_songs(app, &state.library.lock().unwrap());
        state.init_nav(app);
        state
    }

    // ============================================================
    // 初始化 UI 数据
    // ============================================================

    fn init_nav(&self, app: &crate::AppWindow) {
        let model: ModelRc<crate::NavItem> = VecModel::default().into();
        for label in &["全部歌曲", "最近播放", "收藏"] {
            VecModel::from(model.clone()).push(crate::NavItem { label: (*label).into() });
        }
        app.set_nav_items(model.into());
    }

    /// 把 Library.songs 刷到 Slint 的 [SongData] 列表
    pub fn refresh_songs(app: &crate::AppWindow, lib: &crate::Library) {
        let model: ModelRc<crate::SongData> = VecModel::default().into();
        for song in &lib.songs {
            VecModel::from(model.clone()).push(crate::SongData {
                title: song.title.clone().into(),
                path: song.path.clone().into(),
                artist: "".into(),
                duration: "".into(),
            });
        }
        app.set_songs(model.into());
    }

    // ============================================================
    // 14 个 callback 绑定入口
    // ============================================================

    pub fn register_callbacks(&self, app: &crate::AppWindow) {
        self.bind_search_submitted(app);
        self.bind_scan_folder_clicked(app);
        self.bind_song_selected(app);
        self.bind_song_double_clicked(app);
        self.bind_play_pause_clicked(app);
        self.bind_prev_clicked(app);
        self.bind_next_clicked(app);
        self.bind_volume_changed(app);
        self.bind_seek_changed(app);
        self.bind_open_file_location(app);
        self.bind_nav_item_selected(app);
        self.bind_shuffle_toggled(app);
        self.bind_loop_toggled(app);
    }

    // ============================================================
    // Timer 轮询
    // ============================================================

    pub fn start_timer(&self, app: &crate::AppWindow) -> Timer {
        let app_weak = app.as_weak();
        let rx = self.rx.clone();
        let play_state = self.play_state.clone();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
            let rx_lock = rx.lock().unwrap();
            loop {
                match rx_lock.try_recv() {
                    Ok(PlayerEvent::Finished) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_play_state(crate::PlayState::Stopped);
                            // TODO: 自动下一首
                        }
                    }
                    Ok(PlayerEvent::Tick(pos)) => {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_duration_current(playback::format_duration(pos).into());
                        }
                    }
                    Ok(PlayerEvent::Error(msg)) => eprintln!("播放错误: {}", msg),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
            }
            drop(rx_lock);

            // 读 rodio Sink 进度
            let ps = play_state.lock().unwrap();
            if let Some(ref sink) = ps.sink {
                let pos = sink.get_pos().as_secs_f32();
                let total = sink.len().as_secs_f32();
                if total > 0.0, let Some(app) = app_weak.upgrade() {
                    app.set_progress(pos / total * 100.0);
                    app.set_duration_current(playback::format_duration(pos).into());
                }
            }
        });
        timer
    }

    // ============================================================
    // 每个 callback 对应一个方法
    // ============================================================

    fn bind_search_submitted(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let library = self.library.clone();
        app.on_search_submitted(move |keyword: slint::SharedString| {
            let lib = library.lock().unwrap();
            let kw = keyword.to_lowercase();
            let filtered: Vec<&crate::Song> = lib.songs.iter()
                .filter(|s| s.title.to_lowercase().contains(&kw))
                .collect();
            let model: ModelRc<crate::SongData> = VecModel::default().into();
            for song in &filtered {
                VecModel::from(model.clone()).push(crate::SongData {
                    title: song.title.clone().into(),
                    path: song.path.clone().into(),
                    artist: "".into(), duration: "".into(),
                });
            }
            if let Some(app) = app_weak.upgrade() {
                app.set_songs(model.into());
            }
        });
    }

    fn bind_scan_folder_clicked(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let library = self.library.clone();
        app.on_scan_folder_clicked(move || {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                let lib = library.clone();
                let aw = app_weak.clone();
                std::thread::spawn(move || {
                    let mut guard = lib.lock().unwrap();
                    let _ = crate::scan_folder(path.to_str().unwrap_or(""), &mut guard);
                    let _ = crate::save_library(crate::MUSIC_FILE_PATH, &guard);
                    let songs: Vec<crate::Song> = guard.songs.clone();
                    drop(guard);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(app) = aw.upgrade() {
                            Self::refresh_songs(&app, &crate::Library { songs });
                        }
                    });
                });
            }
        });
    }

    fn bind_song_selected(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let library = self.library.clone();
        app.on_song_selected(move |idx: i32| {
            let lib = library.lock().unwrap();
            if let Some(song) = lib.songs.get(idx as usize) {
                if let Some(app) = app_weak.upgrade() {
                    app.set_current_song_title(song.title.clone().into());
                    app.set_current_song_path(song.path.clone().into());
                    // detail-visible 由 app-window.slint 中
                    //   detail-visible: root.current-song-index >= 0
                    // 自动计算
                }
            }
        });
    }

    fn bind_song_double_clicked(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let library = self.library.clone();
        let play_state = self.play_state.clone();
        let tx = self.tx.clone();
        app.on_song_double_clicked(move |idx: i32| {
            if let Some(app) = app_weak.upgrade() {
                app.set_current_song_index(idx);
            }
            playback::start_playing(&library, &play_state, &tx, idx as usize);
        });
    }

    fn bind_play_pause_clicked(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let play_state = self.play_state.clone();
        app.on_play_pause_clicked(move || {
            let mut ps = play_state.lock().unwrap();
            match ps.status {
                PlayStatus::Playing => {
                    if let Some(ref sink) = ps.sink { sink.pause(); }
                    ps.status = PlayStatus::Paused;
                    if let Some(app) = app_weak.upgrade() {
                        app.set_play_state(crate::PlayState::Paused);
                    }
                }
                PlayStatus::Paused => {
                    if let Some(ref sink) = ps.sink { sink.play(); }
                    ps.status = PlayStatus::Playing;
                    if let Some(app) = app_weak.upgrade() {
                        app.set_play_state(crate::PlayState::Playing);
                    }
                }
                PlayStatus::Stopped => {} // 未选歌时忽略
            }
        });
    }

    fn bind_prev_clicked(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let library = self.library.clone();
        let play_state = self.play_state.clone();
        let tx = self.tx.clone();
        app.on_prev_clicked(move || {
            let current = play_state.lock().unwrap().current_song_idx;
            let total = library.lock().unwrap().songs.len();
            if total == 0 { return; }
            let prev = (current + total - 1) % total;
            if let Some(app) = app_weak.upgrade() {
                app.set_current_song_index(prev as i32);
            }
            playback::start_playing(&library, &play_state, &tx, prev);
        });
    }

    fn bind_next_clicked(&self, app: &crate::AppWindow) {
        let app_weak = app.as_weak();
        let library = self.library.clone();
        let play_state = self.play_state.clone();
        let tx = self.tx.clone();
        app.on_next_clicked(move || {
            let current = play_state.lock().unwrap().current_song_idx;
            let total = library.lock().unwrap().songs.len();
            if total == 0 { return; }
            let next = (current + 1) % total;
            if let Some(app) = app_weak.upgrade() {
                app.set_current_song_index(next as i32);
            }
            playback::start_playing(&library, &play_state, &tx, next);
        });
    }

    fn bind_volume_changed(&self, app: &crate::AppWindow) {
        let play_state = self.play_state.clone();
        app.on_volume_changed(move |vol: f32| {
            if let Some(ref sink) = play_state.lock().unwrap().sink {
                sink.set_volume(vol / 100.0);
            }
        });
    }

    fn bind_seek_changed(&self, app: &crate::AppWindow) {
        let play_state = self.play_state.clone();
        app.on_seek_changed(move |percent: f32| {
            if let Some(ref sink) = play_state.lock().unwrap().sink {
                let total = sink.len().as_secs_f32();
                let target = std::time::Duration::from_secs_f32(total * percent / 100.0);
                sink.try_seek(target).ok();
            }
        });
    }

    fn bind_open_file_location(&self, app: &crate::AppWindow) {
        app.on_open_file_location(move |path: slint::SharedString| {
            let _ = open::that(path.as_str());
        });
    }

    fn bind_nav_item_selected(&self, app: &crate::AppWindow) {
        app.on_nav_item_selected(move |idx: i32| {
            eprintln!("导航切换: {}", idx); // TODO: 按导航过滤列表
        });
    }

    fn bind_shuffle_toggled(&self, app: &crate::AppWindow) {
        app.on_shuffle_toggled(move |on: bool| {
            println!("随机: {}", on);
        });
    }

    fn bind_loop_toggled(&self, app: &crate::AppWindow) {
        app.on_loop_toggled(move |on: bool| {
            println!("循环: {}", on);
        });
    }
}
```

> ⚠️ **关于 `app` 参数**：`bind_xxx` 方法的第二个参数是 `app: &AppWindow`，用它来注册 `app.on_xxx(...)` 回调。如果回调闭包中需要更新 UI（调 `set_yyy`），必须用 `app.as_weak()` 捕获弱引用避免循环引用。上面的 `bind_prev_clicked`、`bind_song_selected` 等就是正确范例。不需要更新 UI 的（如 `volume_changed`、`seek_changed`）可以只捕获 `self.play_state` 等共享数据而不捕获 `app`。

---

### 8.4 `playback.rs` — 播放后台线程 + 消息类型

```rust
// src/player/playback.rs
use std::sync::{Arc, Mutex, mpsc};

/// 播放线程 → 主线程的消息
pub enum PlayerEvent {
    Finished,
    Tick(f32),
    Error(String),
}

/// 运行时播放器状态（被 Arc<Mutex<>> 包裹）
#[derive(Default)]
pub struct PlaybackState {
    pub current_song_idx: usize,
    pub status: PlayStatus,
    pub sink: Option<rodio::Sink>,
    // rodio 的 OutputStream 和 Handle 必须存活，drop 会静音
    pub _stream: Option<rodio::OutputStream>,
    pub _handle: Option<rodio::OutputStreamHandle>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayStatus {
    Stopped,
    Playing,
    Paused,
}

impl Default for PlayStatus {
    fn default() -> Self { Self::Stopped }
}

/// 格式化秒数为 "MM:SS"
pub fn format_duration(secs: f32) -> String {
    let m = (secs as u32) / 60;
    let s = (secs as u32) % 60;
    format!("{:02}:{:02}", m, s)
}

/// 启动播放（新建线程）
pub fn start_playing(
    library: &Arc<Mutex<crate::Library>>,
    play_state: &Arc<Mutex<PlaybackState>>,
    tx: &mpsc::Sender<PlayerEvent>,
    song_idx: usize,
) {
    // 1. 拿路径
    let path = match library.lock().unwrap().songs.get(song_idx) {
        Some(s) => s.path.clone(),
        None => return,
    };

    // 2. 更新状态：切歌后旧的 Sink 被 drop 自然停止
    {
        let mut ps = play_state.lock().unwrap();
        ps.current_song_idx = song_idx;
        ps.status = PlayStatus::Playing;
        ps.sink = None;
    }

    // 3. 开线程
    let tx = tx.clone();
    std::thread::spawn(move || {
        let (_stream, handle) = match rodio::OutputStream::try_default() {
            Ok(s) => s,
            Err(e) => { tx.send(PlayerEvent::Error(e.to_string())).ok(); return; }
        };
        let sink = match rodio::Sink::try_new(&handle) {
            Ok(s) => s,
            Err(e) => { tx.send(PlayerEvent::Error(e.to_string())).ok(); return; }
        };

        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => { tx.send(PlayerEvent::Error(e.to_string())).ok(); return; }
        };
        let source = match rodio::Decoder::new(file) {
            Ok(s) => s,
            Err(e) => { tx.send(PlayerEvent::Error(e.to_string())).ok(); return; }
        };

        // 把 Sink 存回 PlaybackState（Timer 用它读进度）
        // 注意：这里存的是新线程中的 sink，需要通过 channel 传回主线程
        // 但 rodio::Sink 是 Send + Sync，可以直接更新 play_state
        {
            let mut ps = play_state.lock().unwrap();
            ps.sink = Some(sink.clone());
        }

        sink.append(source);
        sink.sleep_until_end();

        tx.send(PlayerEvent::Finished).ok();
    });
}
```

### 8.5 `mod.rs` — 模块重新导出

```rust
// src/player/mod.rs
pub mod state;
pub mod playback;
```

### 8.6 改动前后对比

| 指标 | 旧代码 | 重构后 |
|------|--------|--------|
| `main.rs` 行数 | ~230 行 | **20 行** |
| 总代码量 | ~340 行（内联） | ~380 行（分散但结构化） |
| 每个函数行数 | `main()` 230 行 | 每个 handler **6–15 行** |
| 可测试性 | ❌ 闭包内联无法单独测 | ✅ 每个 `bind_xxx` 可单独验证逻辑 |
| clone 重复次数 | 每个 callback 前 3–4 次 clone | 统一在 `register_callbacks` 中一次 clone |
| 增加新的 callback | 在 main() 里再续 30 行 | 加一个 `bind_xxx` 方法 + 在 `register_callbacks` 里调一下 |

### 8.7 动手步骤

```text
1. 创建 src/player/ 目录
2. 放入 mod.rs + state.rs + playback.rs
3. main.rs 改为 20 行的编排版本
4. cargo check 确认编译通过
5. 按"每个 bind_xxx → cargo run"的节奏逐个实现
```

这样你写的时候不用再面对一个 230 行的 main，每次只关注一个 15 行的 `bind_xxx` 方法。

---

## 9. Cargo.toml 新增依赖

```toml
[dependencies]
# 已有
rodio = "0.22.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
slint = "1.16.1"
thiserror = "2.0.18"

# 新增
rfd = "0.15"                  # 文件对话框（可选）
open = "5"                    # 打开文件管理器（可选）
```

> `crossterm` 可以保留，但 Slint 版本不再需要它。如果要精简依赖可以去掉。

---

## 附录：学习敲代码的节奏建议

| 步骤 | 内容 | 预期用时 |
|------|------|---------|
| 1 | 把 `main.rs` 骨架抄下来，配上全部 callback 绑定 | 1 小时 |
| 2 | 跑 `cargo run`，确认窗口能打开、导航能显示、sheet 能看到 | 30 分钟 |
| 3 | 写 `search-submitted` + `song-selected`，让单击列表能更新详情 | 1 小时 |
| 4 | 写 `double-click` 启动播放线程 + Timer 轮询 | 2 小时 |
| 5 | 写 `play-pause` / `prev` / `next` / `volume` | 1 小时 |
| 6 | 写 `scan-folder` 文件对话框 + 后台扫描 | 1 小时 |
| 7 | 补进度条拖动 seek | 30 分钟 |

**每完成一步都 `cargo run` 验证**，不要等全写完再跑。

---

*文档生成时间：2025-07-13*
*对应代码：app-window.slint + 7 个 .slint 子文件 + lib.rs*
