---
title: Germinal DDD 领域模型
---

## 1. 目标

本文用于定义 Germinal 第一版的领域层模型。

本文只描述领域层内部的限界上下文、聚合、实体、值对象、领域行为、领域规则和领域事件。

本文不描述 Application 层、Use Case、跨上下文编排、端口、仓储、基础设施、crate 拆分或具体渲染实现。

本文同样不描述 `GNativeSDK / DSL` 这一层作者接口。作者接口属于 GNativeApp authoring/runtime 体系，不属于宿主领域层；领域层只从结构化应用运行状态、协议消息和渲染边界开始建模。

## 2. 领域层限界上下文

Germinal 第一版领域层划分为四个限界上下文：

```text
Workspace Context
Shell Runtime Context
Input Command Context
Rendering Context
```

各限界上下文彼此独立建模，互不直接依赖。

跨上下文协作由未来的 Application 层负责组织。

---

## 3. Workspace Context

### 3.1 职责

Workspace Context 负责用户工作空间、标签页和分屏结构。

它只关心：

```text
Window
Workspace
Tab
Pane
PaneTree
```

它不关心：

```text
GShell
PtyHost
GShellMode
GNativeAppMode
TerminalBuffer
RenderCommand
GPUFrame
```

### 3.2 它回答的问题

```text
用户当前有哪些 Workspace？
每个 Workspace 有哪些 Tab？
每个 Tab 如何分屏？
哪个 Tab 当前激活？
哪个 Pane 当前激活？
Pane 的位置和尺寸是什么？
```

### 3.3 Workspace Aggregate

#### Aggregate Root

```text
Workspace
```

#### Entities

```text
Tab
Pane
PaneTree
```

#### Value Objects

```text
WorkspaceId
TabId
PaneId
PaneSplit
PaneRect
ActiveTab
ActivePane
```

#### 领域行为

```text
Workspace.open_tab()
Workspace.close_tab(tab_id)
Workspace.activate_tab(tab_id)

Tab.split_pane(pane_id, direction)
Tab.close_pane(pane_id)
Tab.activate_pane(pane_id)
Tab.resize_pane(pane_id, delta)
```

#### 领域规则

```text
一个 Workspace 可以有多个 Tab
一个 Workspace 必须有一个 ActiveTab
一个 Tab 可以有多个 Pane
一个 Tab 必须有一个 ActivePane
Pane 只表示分屏区域，不表示运行单元
Pane 不持有 GShellId
Workspace Context 不知道 GShell 的存在
```

#### 领域事件

```text
WorkspaceCreated
TabOpened
TabClosed
TabActivated
PaneSplit
PaneClosed
PaneActivated
PaneResized
```

---

## 4. Shell Runtime Context

### 4.1 职责

Shell Runtime Context 负责 GShell 的运行状态、传统 PTY 能力、GShell 模式切换和结构化应用运行状态。

它只关心：

```text
GShell
PtyHost
GShellMode
PtyMode
GNativeAppMode
GShellProtocol
GNativeAppInstance
```

它不关心：

```text
Workspace
Tab
Pane
PaneTree
ActivePane
PaneLayer
WindowFrame
```

### 4.2 它回答的问题

```text
GShell 如何启动？
GShell 如何默认进入 PtyMode？
PtyHost 如何运行传统 Shell / CLI / TUI？
GNativeApp 如何通过协议进入 GNativeAppMode？
GNativeApp 退出后如何返回 PtyMode？
当前 GShellMode 输出什么领域结果？
```

### 4.3 GShell Aggregate

#### Aggregate Root

```text
GShell
```

#### Entities

```text
PtyHost
GNativeAppMode
GNativeAppInstance
```

#### Value Objects

```text
GShellId
GShellMode
ProtocolMessage
ProtocolMessageKind
ProtocolPayload
ModeSwitchReason
AppDescriptor
```

#### Output Objects

```text
CurrentModeOutput
PtyModeOutput
GNativeAppModeOutput
```

#### 领域行为

```text
GShell.start_pty_host(shell_config)
GShell.dispatch_key_event(event)
GShell.dispatch_pointer_event(event)
GShell.handle_protocol_message(message)
GShell.enter_native_app_mode(app_descriptor)
GShell.exit_native_app_mode(reason)
GShell.current_mode_output()
GShell.stop()
```

#### 领域规则

```text
一个 GShell 默认持有一个 PtyHost
一个 GShell 默认处于 PtyMode
一个 GShell 同一时刻只能处于一种 GShellMode
GNativeAppMode 必须由 GShellProtocol 显式进入
GNativeAppMode 退出后必须返回 PtyMode
GShell 不做智能识别
PtyHost 与 GNativeAppInstance 内部状态互相隔离
GNativeAppInstance 只在 GNativeAppMode 期间存在
```

#### 领域事件

```text
GShellCreated
GShellStopped
PtyHostStarted
GShellModeChanged
AppModeEntered
AppModeExited
ProtocolMessageReceived
ModeOutputProduced
```

---

### 4.4 PtyHost Model

PtyHost 是 GShell 默认持有的传统 PTY 组件。

#### Entities

```text
PtyProcess
TerminalBuffer
TerminalCursor
Scrollback
Selection
```

#### Value Objects

```text
TerminalCell
CellStyle
GridSize
CursorPosition
TerminalInputSequence
TerminalRenderBatch
```

#### 领域行为

```text
PtyHost.start_shell(shell_config)
PtyHost.write_input(sequence)
PtyHost.read_output()
PtyHost.update_terminal_buffer(bytes)
PtyHost.produce_terminal_render_batch()
PtyHost.resize(grid_size)
```

#### 领域规则

```text
PtyHost 不产生 UiTree
PtyHost 输出 TerminalBuffer
PtyMode 渲染边界是 TerminalRenderBatch
PtyHost 是 GShell 默认兼容能力
PtyHost 可以接收 GShellProtocol 消息
```

---

### 4.5 GShellProtocol Model

GShellProtocol 是 GShell 与支持 Germinal 协议的应用之间的显式控制协议。

#### Value Objects

```text
ProtocolMessage
ProtocolMessageKind
ProtocolPayload
AppDescriptor
```

#### ProtocolMessageKind

```text
EnterNativeAppMode
UpdateNativeAppUi
ExitNativeAppMode
AppHeartbeat
AppError
```

#### 领域行为

```text
GShellProtocol.parse(bytes)
GShellProtocol.validate(message)
GShellProtocol.to_domain_event(message)
```

#### 领域规则

```text
进入 GNativeAppMode 必须依赖 EnterNativeAppMode
退出 GNativeAppMode 必须依赖 ExitNativeAppMode 或 App 进程退出
协议消息必须可序列化
协议消息必须显式表达意图
协议不做智能识别
协议不传输 UiTree 作为远程边界
```

---

### 4.6 GNativeApp Model

GNativeApp 是运行在 GNativeAppMode 中的结构化应用模型。

`GNativeSDK / DSL` 属于作者接口，不直接进入领域层；领域层只关心应用运行态中的 `UiTree / UiNode / UiFocus / RenderCommand` 等结构化输出模型。

#### Entities

```text
GNativeAppInstance
AppState
UiTree
UiNode
UiFocus
```

#### Value Objects

```text
AppInstanceId
UiNodeId
LayoutConstraint
UiRect
RenderCommand
```

#### 领域行为

```text
GNativeAppInstance.handle_key_event(event)
GNativeAppInstance.handle_pointer_event(event)
GNativeAppInstance.update_state(command)
GNativeAppInstance.build_ui_tree()
GNativeAppInstance.layout_ui_tree()
GNativeAppInstance.produce_render_commands()
GNativeAppInstance.request_exit_native_app_mode()
```

#### 领域规则

```text
GNativeAppInstance 不依赖 TerminalBuffer
GNativeAppInstance 输出 UiTree
GNativeAppMode 渲染边界是 RenderCommand
UiTree 不作为远程协议边界
RenderCommand 是结构化 UI 输出到 GShell renderer/compositor 的渲染边界
GNativeAppInstance 生命周期受 GNativeAppMode 约束
```

---

## 5. Input Command Context

### 5.1 职责

Input Command Context 负责输入事件、命令绑定、命令解析和输入目标选择。

它只关心：

```text
KeyEvent
PointerEvent
Command
CommandBinding
CommandRegistry
InputTarget
```

它不关心：

```text
Workspace
Tab
Pane
GShell
PtyHost
GNativeAppInstance
WindowFrame
```

### 5.2 它回答的问题

```text
一个输入事件是否能解析成命令？
一个命令需要什么按键触发？
当前输入目标是什么？
事件是否应被命令系统消费？
```

### 5.3 InputSession Aggregate

#### Aggregate Root

```text
InputSession
```

#### Entities

```text
InputTarget
CommandBinding
CommandRegistry
```

#### Value Objects

```text
InputTargetId
KeyEvent
PointerEvent
Command
CommandId
KeyChord
CommandScope
```

#### 领域行为

```text
InputSession.register_command(command)
InputSession.bind_key(command_id, key_chord)
InputSession.resolve_command(event)
InputSession.activate_input_target(target_id)
InputSession.current_input_target()
```

#### 领域规则

```text
键盘是一等输入
鼠标是辅助输入
全局命令优先于普通输入
未被命令系统消费的输入可以继续交给当前 InputTarget
Input Command Context 不知道 InputTarget 背后是什么对象
Input Command Context 不直接调用 GShell
```

#### 领域事件

```text
CommandRegistered
CommandBindingChanged
CommandTriggered
InputTargetActivated
InputEventConsumed
InputEventForwarded
```

---

## 6. Rendering Context

### 6.1 职责

Rendering Context 负责描述可绘制输出、PaneLayer、WindowFrame 和最终 GPUFrame 的领域概念。

它只关心：

```text
TerminalRenderBatch
RenderCommand
PaneLayer
WindowFrame
GPUFrame
```

它不关心：

```text
Workspace
Tab
Pane
GShell
PtyHost
GNativeAppInstance
UiTree
TerminalBuffer
```

### 6.2 它回答的问题

```text
终端渲染批次如何成为可合成的 Layer？
结构化渲染命令如何成为可合成的 Layer？
多个 Layer 如何组成 WindowFrame？
WindowFrame 如何表达一帧输出？
```

### 6.3 WindowFrame Aggregate

#### Aggregate Root

```text
WindowFrame
```

#### Entities

```text
PaneLayer
```

#### Value Objects

```text
LayerId
LayerRect
TerminalRenderBatch
RenderCommand
GPUFrame
```

#### 领域行为

```text
WindowFrame.compose_from_layers(layers)
WindowFrame.replace_layer(layer_id, layer)
WindowFrame.remove_layer(layer_id)
WindowFrame.output_gpu_frame()
```

#### 领域规则

```text
TerminalRenderBatch 可以形成一个 PaneLayer
RenderCommand 可以形成一个 PaneLayer
WindowFrame 由多个 PaneLayer 合成
Rendering Context 不理解 PtyHost 的业务状态
Rendering Context 不理解 GNativeAppInstance 的业务状态
Rendering Context 不直接访问 UiTree
Rendering Context 不直接访问 TerminalBuffer
```

#### 领域事件

```text
PaneLayerProduced
PaneLayerReplaced
PaneLayerRemoved
WindowFrameComposed
GPUFrameProduced
```

---

## 7. 领域层结构总结

```text
Workspace Context
└── Workspace
    └── Tab
        └── Pane

Shell Runtime Context
└── GShell
    ├── PtyHost
    └── GShellMode
        ├── PtyMode
        └── GNativeAppMode
            └── GNativeAppInstance

Input Command Context
└── InputSession
    ├── CommandRegistry
    └── InputTarget

Rendering Context
└── WindowFrame
    └── PaneLayer
```

## 8. 关键结论

第一版领域层建模结论：

```text
Workspace Context 只负责工作空间组织
Shell Runtime Context 只负责 GShell 运行与模式切换
Input Command Context 只负责输入命令模型
Rendering Context 只负责帧输出模型
```

各限界上下文彼此无感。

不在领域层表达：

```text
PaneId -> GShellId 绑定
InputTargetId -> GShellId 绑定
GShell 输出 -> PaneLayer 绑定
Workspace + ShellRuntime 的创建编排
ShellRuntime + Rendering 的渲染编排
Port
Repository
Infrastructure
UseCase
Application Service
```

这些内容应放到未来的 Application 层设计中。
