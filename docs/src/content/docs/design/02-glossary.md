---
title: Germinal 术语表
---

## Germinal

Germinal 是面向开发者工作流的键盘优先结构化 UI 平台。

它负责管理 Window、Workspace、Tab、Pane、GShell、输入事件、布局、渲染、帧合成和应用承载。

Germinal 中每个 Pane 承载一个 GShell。GShell 默认运行在 PtyMode，并通过 PtyHost 兼容传统 Shell、CLI 和 TUI 程序。当支持 Germinal 协议的 GNativeApp 被启动时，GShell 通过明确协议进入 GNativeAppMode；GNativeApp 退出后，GShell 返回 PtyMode。

## Window

操作系统级窗口。

一个 Window 承载一个 Germinal 实例的可视区域。

## Workspace

工作空间。

用于组织一组相关的 Tab，通常对应一个任务上下文。

这里的“任务”指用户当前正在进行的一项工作或目标，例如开发某个功能、排查一个问题、处理一次运维操作、撰写文档，或进行一次代码审查。

一个 Workspace 可以集中保存完成该任务所需的终端、工具视图和应用状态，从而与其他任务相互隔离。

## Tab

标签页。

Workspace 内的一级切换单位，可包含多个 Pane。

## Pane

面板。

Tab 内的分屏区域。

一个 Pane 承载一个 GShell。

多个 GShell 通过多个 Pane 组织。

## GShell

Graphical Shell。

Pane 中的统一运行单元，负责管理输入、命令分发、PtyHost、当前 GShellMode、协议切换和渲染桥接。

一个 GShell 默认启动 PtyHost，并处于 PtyMode。

当 PtyHost 中运行的程序通过 Germinal 协议请求进入结构化应用模式时，GShell 进入 GNativeAppMode。

当 GNativeApp 退出或发送退出协议时，GShell 返回 PtyMode。

GShellMode 的切换必须由明确协议触发，不做智能猜测。

## GShellMode

GShell 当前的运行模式。

GShellMode 可以是：

- PtyMode
- GNativeAppMode

同一时刻一个 GShell 只能处于一种模式。

## PtyMode

GShell 的默认模式。

PtyMode 使用 PtyHost 的 TerminalBuffer 作为输出来源，用于显示传统 Shell、CLI 和 TUI 程序。

### PtyHost

PtyHost 用于运行传统 Shell、CLI 和 TUI 程序。

PtyHost 不输出结构化 UiTree，而是输出传统终端状态。

PtyHost 在 GShell 生命周期内默认存在，用于提供传统终端兼容能力。

### TerminalBuffer

传统终端缓冲区。

由 PtyHost 产生，表示字符 Cell、样式、光标、选区和滚动历史等终端状态。

### TerminalCell

终端字符单元。

表示终端网格中的一个 Cell，通常包含字符、前景色、背景色和样式属性。

### TerminalCursor

终端光标。

表示 PtyHost 当前字符输入位置。

TerminalCursor 不等同于 GNativeApp 的 UiFocus。

### TerminalRenderer

终端渲染器。

负责把 TerminalBuffer 转换为适合 GPU 绘制的终端渲染批次。

### TerminalRenderBatch

终端渲染批次。

由 TerminalRenderer 生成，用于高效绘制字符网格、字形、背景色、选区和光标。

典型批次包括：

- Glyph Batch
- Cell Background Batch
- Selection Batch
- Cursor Batch

### PtyHost Rendering Path

PtyHost 的一帧渲染路径：

```text
TerminalBuffer
-> TerminalRenderer
-> TerminalRenderBatch
-> PaneLayer
```

PtyHost 不经过 GNativeApp 的 UiTree / RenderCommand 路径。

## GShellProtocol

GShell 与支持 Germinal 的结构化应用之间的控制协议。

GShellProtocol 用于显式请求模式切换，例如：

- enter-native-app-mode
- update-native-app-ui
- exit-native-app-mode

GShell 不通过智能识别判断一个进程是否是 GNativeApp，而是依赖 GShellProtocol 的明确消息。

## GNativeAppMode

GShell 的结构化应用模式。

GNativeAppMode 由 GNativeApp 通过 GShellProtocol 显式进入。

在 GNativeAppMode 中，输入事件主要分发给 GNativeApp，渲染输出主要来自 GNativeApp 的 RenderCommand。

GNativeAppMode 退出后，GShell 返回 PtyMode。

## GNativeApp

运行在 GNativeAppMode 中的结构化应用。

GNativeApp 不依赖字符网格，而是输出结构化 UI。

GNativeApp 通过 GShellProtocol 请求进入和退出 GNativeAppMode。

GNativeApp 的开发者通常直接使用 `GNativeSDK` 提供的 DSL 或声明式 API，而不是手写 UiTree 或 RenderCommand。

## GNativeSDK

GNativeApp 作者侧 SDK。

它提供 DSL、运行时、状态驱动更新和协议发送能力。GNativeSDK 负责把作者写的高层声明式 UI 展开为 UiTree，并进一步生成 RenderCommand。

## DSL

GNativeSDK 面向应用作者的高层声明式 UI 描述层。

DSL 用于表达布局、组件、状态和交互，不直接暴露 GPU 后端对象，也不要求应用作者手写底层渲染命令。

### UiTree

结构化 UI 树。

由 GNativeSDK 或 GNativeApp 运行时产生，表示当前结构化界面状态。

UiTree 是结构化 UI 的内部 IR，用于表达节点层级、语义和属性，不是最终渲染协议边界。

### UiNode

结构化 UI 节点。

UiTree 中的基本节点，用于描述界面元素、层级、状态、布局约束和交互语义。

### UiFocus

结构化 UI 焦点。

表示 GNativeApp 内部当前接收键盘输入的 UiNode。

UiFocus 不等同于 PtyHost 的 TerminalCursor。

### AppLayout

结构化 UI 布局阶段。

AppLayout 负责计算 UiNode 的几何位置、尺寸和约束结果。

### RenderCommand

结构化 UI 渲染命令。

由布局后的 UiTree 生成，用于描述矩形、文字、图片、视频、裁剪、层叠、变换等高层绘制操作。

RenderCommand 是 GNativeApp 输出到 GShell renderer/compositor 的渲染 IR 和协议边界，不是底层 GPU 命令对象。

### RenderCommand Transport

RenderCommand 的传输链路。

在本地模式下，推荐由 GNativeApp 先通过 PTY / GShellProtocol 显式请求进入 GNativeAppMode，再建立专用 transport 发送 RenderCommand 帧。

推荐约束是：

- PTY 只负责 shell 兼容、启动入口和模式切换握手。
- RenderCommand 走独立命令通道，不长期混在 PTY 文本流里。
- 大资源不直接塞进 RenderCommand，而是通过 `resource_id` 被命令引用。
- GShell renderer/compositor 消费 RenderCommand 和资源更新后，再生成 PaneLayer。

### GNativeApp Rendering Path

GNativeApp 的一帧输出主链路：

```text
DSL / GNativeSDK
-> UiTree
-> AppLayout
-> RenderCommand
-> transport
-> GShell renderer/compositor
-> PaneLayer
```

GNativeApp 不经过 PtyHost 的 TerminalBuffer / TerminalRenderBatch 路径。

## ActiveGShell

当前激活的 GShell。

ActiveGShell 表示当前接收键盘输入的 GShell。

Germinal 先决定 ActiveGShell，再根据 GShellMode 把 KeyEvent 分发给 PtyMode 或 GNativeAppMode。

## KeyEvent

键盘事件。

Germinal 的一等输入事件，用于驱动快捷键、命令和应用交互。

## PointerEvent

指针事件。

鼠标、触控板等辅助输入事件。

## Command

命令。

用户通过快捷键、命令面板或应用逻辑触发的动作。

## GerminalLayout

Germinal 的布局系统。

负责计算 Window、Workspace、Tab、Pane 的位置与尺寸。

## PaneLayer

Pane 的渲染层。

每个 Pane 最终生成一个 PaneLayer。

PtyHost 和 GNativeApp 分别在各自的 Pane 生成 PaneLayer。

## Renderer

渲染器。

负责把不同来源的渲染数据绘制并合成为最终画面。

Renderer 需要支持两类路径：

```text
PtyHost -> TerminalRenderBatch -> PaneLayer
GNativeApp -> RenderCommand -> PaneLayer
```

## WindowFrame

窗口最终帧。

由 Germinal 收集所有 PaneLayer 后生成。

典型结构：

```text
WindowFrame
├── PaneLayer #1
└── PaneLayer #2
```

## GPUFrame

一帧 GPU 渲染结果。

表示一次完整绘制后提交给窗口系统显示的画面。

## Remote Boundary

远程能力边界。

第一版只预留事件、状态和渲染协议边界，不实现远程运行和网络传输。

远程输出边界应是：

```text
PtyMode       -> TerminalRenderBatch
GNativeAppMode -> RenderCommand
```

UiTree 不作为远程协议边界。
