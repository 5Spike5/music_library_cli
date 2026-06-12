# Slint 音乐播放器 · UI 设计文档

> **状态**：UI 层已完成（8 个 `.slint` 文件），Rust 侧尚未对接。  
> **本文档**：解释架构设计理由、每个组件的 Slint 语法要点、以及 Rust 侧需要实现的函数签名与对接方案。

---

## 目录

1. [架构总览](#1-架构总览)
2. [为什么这样设计](#2-为什么这样设计)
3. [文件结构](#3-文件结构)
4. [核心 Slint 语法讲解](#4-核心-slint-语法讲解)
5. [区域设计详解](#5-区域设计详解)
   - [5.1 Header — 顶部栏](#51-header--顶部栏)
   - [5.2 Nav — 左侧导航栏](#52-nav--左侧导航栏)
   - [5.3 List — 中间歌曲列表](#53-list--中间歌曲列表)
   - [5.4 Detail — 右侧详情区](#54-detail--右侧详情区)
   - [5.5 Footer — 底部播放控制栏](#55-footer--底部播放控制栏)
6. [Rust 侧对接指南](#6-rust-侧对接指南)
7. [Rust 待实现函数清单](#7-rust-待实现函数清单)
8. [常见坑与注意事项](#8-常见坑与注意事项)

---

## 1. 架构总览

```
                    app-window.slint (根窗口)
                    ┌──────────┐
                    │  Header  │  ← header.slint
                    ├──────────┤
                    │ Nav│List │  ← nav.slint + list.slint + detail.slint
                    │    │Det. │
                    ├──────────┤
                    │  Footer  │  ← footer.slint
                    └──────────┘
        共享层: common.slint (struct/enum) + theme.slint (color tokens)
```

**数据流方向**：

```
Rust 侧                             Slint 侧
═══════                             ═══════
app.set_songs(model)  ──────────►   List 组件显示歌曲列表
app.set_song_title(s)  ────────►   Detail 组件显示当前歌曲信息
app.set_progress(50.0)  ───────►   Detail 进度条 ← Slider value <=> progress

用户操作触发 callback ◄──────────
  app.on_song_selected(|idx| ..)    List 组件: 用户点击歌曲行
  app.on_play_pause_clicked(|| ..)  Footer 组件: 用户按播放按钮
  app.on_search_submitted(|kw| ..)  Header 组件: 用户回车搜索
```

---

## 2. 为什么这样设计

### 2.1 为什么「先 UI 后 Rust」

| 原因 | 说明 |
|------|------|
| 接口先行 | 先把 `callback` 和 `property` 写好，Rust 代码就有明确的"函数签名合约" |
| 减少返工 | 如果先写 Rust 逻辑，后面对接 Slint 时需要大量回调适配 |
| 可视化验证 | UI 写完就能 `cargo run` 看布局，不需要等逻辑写好 |

### 2.2 为什么拆分 8 个文件

| 理由 | 效果 |
|------|------|
| 职责单一 | 每个文件只做一个区域，修 Header 不影响 Footer |
| 独立测试 | 可以单独 import 一个组件做调试 |
| 团队协作 | 多人可以同时写不同区域 |
| Slint 的 import 模型 | `slint_build::compile("app-window.slint")` 自动追踪 import 链 |

### 2.3 为什么用 Unicode 图标而不是第三方图标库

- Slint 没有内置图标库，也没有成熟的第三方图标集成方案
- Unicode 符号（▶ ⏸ ⏮ ⏭ 🔀 🔁 🔊 📁 📂 🔍 🎵）在所有平台显示一致
- 零外部依赖、零加载延迟
- 音乐播放器场景下完全够用

### 2.4 布局比例依据

```
Header  10%  → 够放标题 + 搜索 + 设置按钮
Nav     18%  → 3-5 个导航项 + 底部统计
List    52%  → 主视图，最大化信息密度
Detail  30%  → 封面 + 歌曲信息 + 进度条，不挤也不空
Footer  12%  → 按钮 + 音量条，始终可见
```

参考了 Apple Music / Spotify Desktop 的经典三栏布局。

---

## 3. 文件结构

```
ui/
├── common.slint         ← 共享数据结构：NavItem, SongData, PlayState enum
├── theme.slint          ← 全局主题：ThemeMode enum + Theme global (14 color tokens)
├── header.slint         ← 顶部栏组件
├── nav.slint            ← 左侧导航组件
├── list.slint           ← 中间歌曲列表组件
├── detail.slint         ← 右侧详情组件
├── footer.slint         ← 底部控制栏组件
└── app-window.slint     ← 根窗口（组装所有组件 + 声明所有 callback/property 接口）
```

**编译入口**：`build.rs` 中的 `slint_build::compile("ui/app-window.slint")` 会自动发现 `import` 的子文件。

---

## 4. 核心 Slint 语法讲解

### 4.1 `property` 声明

```slint
// Slint 侧声明（app-window.slint）
in-out property <int> current-song-index: -1;
//                  ^^^              ^^^
//                类型             默认值

// Rust 侧读写
app.set_current_song_index(5);       // Slint 自动生成 set_xxx()
let idx = app.get_current_song_index(); // Slint 自动生成 get_xxx()
```

| 修饰符 | 含义 | Rust 侧可用 |
|--------|------|-----------|
| `in property` | 只写 | `set_xxx()` |
| `out property` | 只读 | `get_xxx()` |
| `in-out property` | 读写 | `set_xxx()` + `get_xxx()` |

### 4.2 `callback` 声明

```slint
// Slint 侧声明
callback song-selected(int);
//       ^^^^^^^^^^^^^ ^^^
//       回调名        参数类型

// Rust 侧绑定
app.on_song_selected(move |idx: i32| {
    // 用户点击列表项时执行
    let song = lib.lock().unwrap().songs[idx as usize].clone();
    app.set_current_song_title(song.title.into());
});
```

### 4.3 `<=>` 双向绑定

```slint
// Slint 侧
Slider {
    value <=> root.progress;  // Slider 拖动 → progress 自动同步
}
```

```rust
// Rust 侧写
app.set_progress(50.0);  // 更新 Slider
// Rust 侧读
let p = app.get_progress();
```

**关键**：`<=>` 是 `in-out property` 的引用绑定，两端任一变化都会自动同步对方。

### 4.4 `for` 循环

```slint
for song[idx] in songs : Rectangle {
    // song:  当前元素（类型 = SongData）
    // idx:   当前索引（int）
    Text { text: song.title; }
}
```

**Rust 侧填充**：

```rust
use slint::VecModel;
let model: slint::ModelRc<SongData> = slint::VecModel::default().into();
model.push(SongData { title: "晴天".into(), /* ... */ });
app.set_songs(model.into());
```

### 4.5 `if` 条件渲染

```slint
if songs.length == 0 : Rectangle {
    Text { text: "📂 曲库为空"; }
}
if songs.length > 0 : Flickable {
    // 渲染列表
}
```

> Slint 会根据 `songs` 数组的模型变化自动切换两种状态。

### 4.6 `TouchArea` 与 `clicked` / `double-clicked`

```slint
ta := TouchArea {
    clicked => { root.song-selected(idx); }
    double-clicked => { root.song-double-clicked(idx); }
    // → 子元素自动获得点击检测
    Rectangle {
        Text { text: "点我"; }
    }
}
```

**命名 TouchArea**：如果需要读取 TouchArea 自身的属性（如 `pressed`、`has-hover`），必须给它命名（`ta :=`）。

### 4.7 `Slider` 的 `changed` 回调

```slint
Slider {
    minimum: 0.0;
    maximum: 100.0;
    value <=> root.progress;
    changed(percent) => {
        root.seek-changed(percent);  // 用户拖拽时触发 seek
    }
}
```

> `changed` 在用户交互时触发；`value <=> root.progress` 确保程序修改 progress 时滑块也跟随。

### 4.8 `LineEdit` 的 `accepted` 回调

```slint
search := LineEdit {
    placeholder-text: "搜索歌曲…";
    accepted => { root.search-submitted(search.text); }
}
```

> `accepted` = 用户按 Enter；`search.text` 是当前输入的文本。

### 4.9 `import` 语法

```slint
// 从其他 .slint 文件导入类型/组件
import { SongData, PlayState } from "common.slint";
import { Theme, ThemeMode } from "theme.slint";
import { Slider, LineEdit } from "std-widgets.slint";
```

> `std-widgets.slint` 是 Slint 内置标准控件库，路径固定。

---

## 5. 区域设计详解

### 5.1 Header — 顶部栏

```slint
// header.slint
export component Header {
    callback search-submitted(string);  // 用户回车搜索
    callback scan-folder-clicked();     // 用户点扫描按钮
    in property <string> window-title: "Music Library";
}
```

**设计理由**：

| 元素 | 为什么放这里 | Slint 语法 |
|------|------------|-----------|
| 标题 `window-title` | 顶部左对齐是桌面应用常规 | `Text` + `font-weight: 700` |
| 搜索框 `LineEdit` | 搜索是最高频操作，放顶部最易发现 | `import { LineEdit } from "std-widgets.slint"` |
| 扫描图标 `📁` | 紧邻搜索框，功能相关 | 未来绑定 `scan-folder-clicked` |
| 4 色盘按钮 | 最小化的主题切换 UI | `TouchArea` + `Rectangle` 圆形色块 + `border-width` 条件高亮 |

**预留 Rust 接口**：

| Slint 声明 | Rust 绑定 | 用途 |
|-----------|----------|------|
| `callback search-submitted(string)` | `app.on_search_submitted(\|kw\| { ... })` | 过滤歌曲列表 |
| `callback scan-folder-clicked()` | `app.on_scan_folder_clicked(\|\| { ... })` | 弹出文件夹选择器，扫描 MP3 |

---

### 5.2 Nav — 左侧导航栏

```slint
// nav.slint
export component Nav {
    in-out property <int> current-nav-index: 0;
    in property <[NavItem]> nav-items;
    callback nav-item-selected(int);
}
```

**设计理由**：

| 元素 | 为什么 | Slint 语法 |
|------|--------|-----------|
| `for nav-item[idx] in nav-items` | 数据驱动，所有导航项由 Rust 侧 VecModel 控制 | `for-in` 循环 |
| 选中高亮 `accent.with-alpha(0.25)` | 当前选中项用主题色半透明背景 | `condition ? valueA : valueB` |
| 底部统计 `"共 0 首歌"` | 菜单底部显示曲库统计是主流做法 | 纯 Text |

**预留 Rust 接口**：

| Slint 声明 | Rust 绑定 | 用途 |
|-----------|----------|------|
| `in property <[NavItem]> nav-items` | `app.set_nav_items(model.into())` | 填充导航项 |
| `callback nav-item-selected(int)` | `app.on_nav_item_selected(\|idx\| { ... })` | 切换视图（未来扩展） |

---

### 5.3 List — 中间歌曲列表

```slint
// list.slint
export component List {
    in-out property <int> current-song-index: -1;
    in property <[SongData]> songs;
    callback song-selected(int);
    callback song-double-clicked(int);
}
```

**设计理由**：

| 元素 | 为什么 | Slint 语法 |
|------|--------|-----------|
| 表头 `# / ♫ / 标题 / 时长` | 固定不可滚动，提供列语义 | `HorizontalLayout` + 固定宽度 |
| 空状态 `"📂 曲库为空"` | 无数据时友好提示 | `if songs.length == 0 : Rectangle` |
| `Flickable` 可滚动 | 歌曲多时自动滚动 | `Flickable { viewport-y: 0px; }` |
| 选中高亮 | 当前播放/选中的行用 accent 色标识 | `Theme.accent.with-alpha(0.15)` |
| 序号 `\{idx + 1}` | 动态计算行号 | Slint 字符串插值 `"\{expr}"` |
| 时长右对齐 | 数字类信息通常右对齐 | `horizontal-alignment: right` |

**预留 Rust 接口**：

| Slint 声明 | Rust 绑定 | 用途 |
|-----------|----------|------|
| `in property <[SongData]> songs` | `app.set_songs(model.into())` | 填充歌曲列表 |
| `callback song-selected(int)` | `app.on_song_selected(\|idx\| { ... })` | 更新 Detail 面板 |
| `callback song-double-clicked(int)` | `app.on_song_double_clicked(\|idx\| { ... })` | 直接播放 |

---

### 5.4 Detail — 右侧详情区

```slint
// detail.slint
export component Detail {
    in-out property <bool> detail-visible: false;
    in property <image> cover-image;
    in property <string> song-title / song-artist / song-path / duration-total / duration-current;
    in-out property <float> progress: 0.0;
    in property <PlayState> play-state;
    callback seek-changed(float);
    callback open-file-location(string);
}
```

**设计理由**：

| 元素 | 为什么 | Slint 语法 |
|------|--------|-----------|
| 封面占位 `🎵` | 先用图标占位，未来从 ID3 标签提取 | `if cover-image.width == 0` 判断是否有图片 |
| 进度条 `Slider` | 拖动跳进度是播放器核心功能 | `value <=> root.progress` + `changed(percent) => seek-changed` |
| 时间显示 `00:00 / 03:45` | 已播/总时长，用户刚需 | `duration-current` + `duration-total` 由 Rust Timer 更新 |
| 打开文件位置 `📂` | 方便用户定位文件 | `callback open-file-location(path)` → Rust 调 `explorer /select,path` |
| `detail-visible` | 未选歌曲时显示提示，选了才显示详情 | `if !root.detail-visible : Rectangle { ... }` |

**预留 Rust 接口**：

| Slint 声明 | Rust 绑定 | 用途 |
|-----------|----------|------|
| `callback seek-changed(float)` | `app.on_seek_changed(\|pos\| { sink.try_seek(...) })` | 跳转播放位置 |
| `callback open-file-location(string)` | `app.on_open_file_location(\|path\| { open::that(...) })` | 资源管理器打开文件 |
| `in-out property <float> progress` | `app.set_progress(50.0)` | 播放进度 0.0~100.0 |

---

### 5.5 Footer — 底部播放控制栏

```slint
// footer.slint
export component Footer {
    in property <string> now-playing-text;
    in property <PlayState> play-state;
    in-out property <float> volume / <bool> shuffle-on / <bool> loop-on;
    callback prev-clicked() / play-pause-clicked() / next-clicked();
    callback shuffle-toggled(bool) / loop-toggled(bool) / volume-changed(float);
}
```

**设计理由**：

| 元素 | 为什么 | Slint 语法 |
|------|--------|-----------|
| 播放按钮 `▶` ↔ `⏸` | 根据 `play-state` 自动切换图标 | `root.play-state == PlayState.Playing ? "⏸" : "▶"` |
| 播放按钮圆形 accent 色 | 最突出的视觉焦点 | `border-radius: 20px` + `background: Theme.accent` |
| 随机/循环 半透明开关 | 非核心功能降低视觉权重 | `opacity: root.shuffle-on ? 1.0 : 0.4` |
| 音量 `Slider` 双向绑定 | 拖动实时更新 | `value <=> root.volume` + `changed(vol) => volume-changed(vol)` |

**预留 Rust 接口**：

| Slint 声明 | Rust 绑定 | 用途 |
|-----------|----------|------|
| `callback play-pause-clicked()` | `app.on_play_pause_clicked(\|\| { sink.play()/pause() })` | 播放/暂停切换 |
| `callback prev-clicked()` / `next-clicked()` | `app.on_prev_clicked(...)` / `app.on_next_clicked(...)` | 切歌 |
| `callback volume-changed(float)` | `app.on_volume_changed(\|vol\| { sink.set_volume(vol) })` | 音量调节 |
| `callback shuffle-toggled(bool)` | `app.on_shuffle_toggled(\|on\| { ... })` | 随机播放 |
| `callback loop-toggled(bool)` | `app.on_loop_toggled(\|on\| { ... })` | 循环播放 |
| `in-out property <float> volume` | `app.set_volume(70.0)` / `app.get_volume()` | 音量双向绑定 |

---

## 6. Rust 侧对接指南

### 6.1 完整对接骨架

```rust
// main.rs — 骨架示意（不是最终代码）
slint::include_modules!();  // build.rs 生成

use std::sync::{Arc, Mutex, mpsc};
use slint::{Timer, TimerMode, VecModel, ModelRc, invoke_from_event_loop};

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    // ===== 初始化数据模型 =====
    let song_model: ModelRc<SongData> = VecModel::default().into();
    let nav_model: ModelRc<NavItem> = VecModel::default().into();

    // 填充导航项
    for label in ["全部歌曲", "最近播放", "收藏"] {
        nav_model.push(NavItem { label: label.into() });
    }
    app.set_nav_items(nav_model.into());
    app.set_songs(song_model.clone().into());  // 初始空列表，后续动态填充

    // ===== 共享状态 =====
    let library: Arc<Mutex<Library>> = Arc::new(Mutex::new(load_library("library.json")?));
    let (tx, rx) = mpsc::channel::<PlayerEvent>();

    // ===== 注册回调 =====
    let lib_for_cb = library.clone();
    app.on_search_submitted(move |keyword: slint::SharedString| {
        let lib = lib_for_cb.lock().unwrap();
        let results: Vec<&Song> = lib.songs.iter()
            .filter(|s| s.title.to_lowercase().contains(&keyword.to_lowercase()))
            .collect();
        // 更新模型
        drop(lib);
        // song_model.set_vec(results);  // 伪代码：需要转为 SongData
    });

    // ... 其余回调 ...

    // ===== 启动事件循环 =====
    app.run()
}
```

### 6.2 关键类型映射

| Slint 类型 | Rust 类型 | 构造/转换 |
|-----------|----------|----------|
| `string` | `slint::SharedString` | `"hello".into()` / `.to_string()` |
| `int` | `i32` | 直接传 |
| `float` | `f32` | 直接传 |
| `bool` | `bool` | 直接传 |
| `[SongData]` | `slint::ModelRc<SongData>` | `VecModel::default().into()` |
| `image` | `slint::Image` | `slint::Image::load_from_path(...)` |

### 6.3 Rust 如何更新 Slint 属性

```rust
// 方式1：直接 set
app.set_current_song_title("晴天".into());
app.set_progress(42.5);

// 方式2：先 get 再修改（in-out 属性）
let idx = app.get_current_song_index();
app.set_current_song_index(idx + 1);

// 方式3：通过模型更新列表
let model: ModelRc<SongData> = app.get_songs();
// 如果需要在 Rust 侧修改 model 内容，持有它的引用：
// model.push(new_song) 或 model.remove(idx)
```

### 6.4 Rust 如何绑定回调

```rust
// 无参数回调
app.on_play_pause_clicked(move || {
    // 访问共享状态
    let mut state = play_state.lock().unwrap();
    *state = match *state {
        PlaybackState::Playing => PlaybackState::Paused,
        PlaybackState::Paused => PlaybackState::Playing,
        _ => PlaybackState::Playing { song_idx: 0 },
    };
});

// 带参数回调
app.on_song_selected(move |idx: i32| {
    println!("用户选择了第 {} 首", idx);
});
```

### 6.5 从后台线程更新 UI

```rust
let app_weak = app.as_weak();  // 必须在主线程获取弱引用
std::thread::spawn(move || {
    // 播放完毕...
    let _ = slint::invoke_from_event_loop(move || {
        let app = app_weak.upgrade().unwrap();
        app.set_now_playing_text("下一首".into());
    });
});
```

### 6.6 Timer 定时更新进度

```rust
let app_weak = app.as_weak();
let timer = Timer::default();
timer.start(TimerMode::Repeated, std::time::Duration::from_millis(200), move || {
    let app = app_weak.upgrade().unwrap();
    // 从 rodio Sink 读取当前位置
    // let pos = sink.get_pos().as_secs_f32();
    // let total = song_duration.as_secs_f32();
    // app.set_progress(pos / total * 100.0);
    // app.set_duration_current(format_duration(pos).into());
});
```

---

## 7. Rust 待实现函数清单

> **顺序建议**：从上到下逐步实现，每实现一个就 `cargo run` 验证。

### 第一优先（让列表显示数据）

| # | 功能 | 涉及的 Slint 接口 | 难度 |
|---|------|------------------|------|
| 1 | 启动时加载 `library.json` → 填充 VecModel | `app.set_songs(model)` | ⭐ |
| 2 | 用户点列表项 → 更新 Detail | `on_song_selected(idx)` → `set_song_title/set_song_path/set_detail_visible` | ⭐ |
| 3 | 搜索回车 → 过滤列表 | `on_search_submitted(kw)` → 重建 model | ⭐⭐ |

### 第二优先（播放控制）

| # | 功能 | 涉及的 Slint 接口 | 难度 |
|---|------|------------------|------|
| 4 | 播放按钮 → 启动 rodio 后台线程 | `on_play_pause_clicked()` | ⭐⭐⭐ |
| 5 | 暂停切换 | `on_play_pause_clicked()` → `sink.pause()/play()` | ⭐⭐ |
| 6 | 上一首/下一首 | `on_prev_clicked()` / `on_next_clicked()` | ⭐⭐ |
| 7 | Timer 更新进度条 + 时间显示 | `set_progress()` / `set_duration_current()` | ⭐⭐ |

### 第三优先（辅助功能）

| # | 功能 | 涉及的 Slint 接口 | 难度 |
|---|------|------------------|------|
| 8 | 音量调节 | `on_volume_changed(vol)` → `sink.set_volume()` | ⭐ |
| 9 | 进度条拖拽 seek | `on_seek_changed(pos)` → `sink.try_seek()` | ⭐⭐ |
| 10 | 随机/循环模式 | `on_shuffle_toggled()` / `on_loop_toggled()` | ⭐ |
| 11 | 打开文件位置 | `on_open_file_location(path)` → `open::that()` | ⭐ |
| 12 | 扫描文件夹 | `on_scan_folder_clicked()` → `native_dialog::FileDialog` | ⭐⭐ |

---

## 8. 常见坑与注意事项

### 8.1 Slint 编译错误排错流程

```
1. cargo check → 看第一个 error
2. 定位到 .slint 行号
3. 检查: 变量名/属性名拼写? import 遗漏? 类型不匹配?
4. 修改 → cargo check → repeat
```

### 8.2 Slint 不支持的特性

| 不支持 | 替代方案 |
|--------|---------|
| `padding: 4px 8px` 双值简写 | 分别写 `padding-left` / `padding-right` / `padding-top` / `padding-bottom` |
| `.to-string()` 方法 | 字符串插值 `"\{expr}"` |
| `min` / `max` (Slider) | 用 `minimum` / `maximum` |
| `touch.pressed` 魔法变量 | 给 TouchArea 命名: `ta := TouchArea { ... }`，用 `ta.pressed` |
| `Flickable` import | 不需要 import，它是内置元素 |

### 8.3 import 遗漏

- `global` 不会自动跨文件可用 → 必须 `import { Theme } from "theme.slint"`
- `std-widgets` 里的控件（`Slider`、`LineEdit`、`Button`）必须显式 import
- 自定义 struct/enum 必须 `import { ... } from "common.slint"`

### 8.4 类型转换注意事项

```rust
// ❌ 错误：直接传 String
app.set_song_title(String::from("晴天"));  // 编译失败

// ✅ 正确：转为 SharedString
app.set_song_title("晴天".into());  // &str → SharedString
app.set_song_title(song.title.clone().into());  // String → SharedString

// VecModel → ModelRc
let model: slint::ModelRc<SongData> = slint::VecModel::default().into();
app.set_songs(model.into());
```

### 8.5 线程安全

- `app.run()` 阻塞主线程 → 播放/IO 必须放到其他线程
- 子线程更新 UI 必须通过 `slint::invoke_from_event_loop()`
- Timer 闭包中必须用 `app.as_weak()` 避免循环引用

---

*文档生成时间：2025-07-13*
*配套代码版本：8 个 .slint 文件，cargo check 通过*
