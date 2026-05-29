---
title: Germinal 大需求分解
---

## 1. 大需求

构建 Germinal：一个键盘优先、命令驱动、以结构化 UI 为主、兼容传统终端的图形化开发者 UI 平台。

Germinal 默认以 PtyHost / PtyMode 运行传统 shell。当用户在 shell 中运行支持 Germinal 协议的 GNativeApp 时，GShell 通过明确协议进入 GNativeAppMode；GNativeApp 退出后，GShell 返回 PtyMode。

## 2. 一级需求

### R1. 窗口与工作区组织

Germinal 需要提供 Window、Workspace、Tab、Pane 的层级组织能力，用于承载多个开发任务和多个 GShell。

### R2. GShell 运行单元

Germinal 需要以 GShell 作为 Pane 中的统一运行单元。

一个 Pane 承载一个 GShell。

一个 GShell 默认启动 PtyHost，并处于 PtyMode。

### R3. GShellMode 模式模型

Germinal 需要支持两种 GShellMode：

- PtyMode
- GNativeAppMode

一个 GShell 同一时刻只能处于一种模式。

模式切换必须由 GShellProtocol 显式触发，不做智能识别。

### R4. 传统终端兼容

Germinal 需要通过 PtyHost 运行传统 Shell、CLI 和 TUI 程序。

### R5. 结构化应用承载

Germinal 需要通过 GNativeAppMode 承载结构化 UI 应用。

GNativeApp 不依赖字符网格。应用作者的主开发接口应是 GNativeSDK / DSL；UiTree、AppLayout 和 RenderCommand 属于内部结构化 UI 链路与宿主渲染边界。

### R6. 键盘优先输入模型

Germinal 需要以 KeyEvent 作为一等输入事件，支持 ActiveGShell 切换、命令触发和不同 GShellMode 内部交互。

### R7. 渲染与帧合成

Germinal 需要支持 PtyMode 和 GNativeAppMode 两条渲染路径，并将多个 PaneLayer 合成为 WindowFrame，最终生成 GPUFrame。

### R8. 远程能力边界

Germinal 第一版不实现远程运行、网络传输和多客户端同步，但架构需要为未来远程能力预留边界。

## 3. 二级需求

### R1. 窗口与工作区组织

#### R1.1 Window

系统需要创建一个操作系统级窗口，作为 Germinal 的可视区域。

#### R1.2 Workspace

系统需要支持多个 Workspace，用于隔离不同任务上下文。

#### R1.3 Tab

每个 Workspace 需要支持多个 Tab。

#### R1.4 Pane

每个 Tab 需要支持多个 Pane。

#### R1.5 Pane 布局

系统需要支持 Pane 的分屏、切换、关闭和尺寸调整。

---

### R2. GShell 运行单元

#### R2.1 GShell 创建

系统需要为每个 Pane 创建一个 GShell。

#### R2.2 GShell 生命周期

系统需要管理 GShell 的创建、激活、挂起和销毁。

#### R2.3 PtyHost 默认启动

每个 GShell 创建后默认启动 PtyHost。

#### R2.4 默认 PtyMode

每个 GShell 默认处于 PtyMode，用于显示 PtyHost 的传统终端输出。

#### R2.5 GShellMode 生命周期

GShell 需要管理当前 GShellMode，并保证同一时刻只能处于一种模式。

---

### R3. GShellMode 模式模型

#### R3.1 PtyMode

PtyMode 使用 PtyHost 的 TerminalBuffer 作为输出来源。

#### R3.2 GNativeAppMode

GNativeAppMode 使用 GNativeApp 的 RenderCommand 作为结构化 UI 输出边界。

#### R3.3 进入 GNativeAppMode

当 PtyHost 中运行的程序发送 `enter-native-app-mode` 协议消息时，GShell 进入 GNativeAppMode。

#### R3.4 退出 GNativeAppMode

当 GNativeApp 退出或发送 `exit-native-app-mode` 协议消息时，GShell 返回 PtyMode。

#### R3.5 非智能识别

系统不通过进程名、输出内容或启发式规则判断 GNativeApp。模式切换必须由 GShellProtocol 明确触发。

---

### R4. 传统终端兼容

#### R4.1 PTY 创建

PtyHost 需要创建并管理 PTY。

#### R4.2 Shell 启动

PtyHost 需要启动用户配置的 Shell。

#### R4.3 TerminalBuffer

PtyHost 需要维护 TerminalBuffer，包括 TerminalCell、样式、光标、选区和滚动历史。

#### R4.4 终端输入

PtyHost 需要把键盘输入转换为传统终端输入序列。

#### R4.5 终端渲染

PtyHost 需要通过 TerminalRenderer 把 TerminalBuffer 转换为 TerminalRenderBatch。

---

### R5. 结构化应用承载

#### R5.1 GNativeApp 生命周期

系统需要管理 GNativeApp 的启动、运行、退出和模式恢复。

#### R5.2 GShellProtocol

GNativeApp 需要通过 GShellProtocol 显式请求进入、更新和退出 GNativeAppMode。

#### R5.3 GNativeSDK / DSL

GNativeApp 作者的主开发入口应是 GNativeSDK / DSL。

应用作者不应被要求直接手写 RenderCommand，也不应直接依赖 GPU 后端对象。

#### R5.4 UiTree

GNativeSDK 或 GNativeApp 运行时需要输出 UiTree。

UiTree 用于表达结构化 UI 节点层级、语义和属性，是内部 IR，不是远程协议边界。

#### R5.5 UiNode

UiTree 由 UiNode 组成，UiNode 需要表达结构、状态、布局约束和交互语义。

#### R5.6 AppLayout

GNativeApp 运行时需要通过 AppLayout 计算 UiNode 的位置和尺寸。

#### R5.7 RenderCommand

GNativeApp 运行时需要把布局后的 UiTree 转换为 RenderCommand。

RenderCommand 是 GNativeApp 输出给 GShell renderer/compositor 的高层绘制语义边界，不是底层 GPU 命令。

#### R5.8 UiFocus

GNativeApp 需要维护内部 UiFocus。

---

### R6. 键盘优先输入模型

#### R6.1 ActiveGShell

系统需要维护当前 ActiveGShell。

#### R6.2 全局快捷键

系统需要支持 Pane 切换、Tab 切换、Workspace 切换和命令触发。

#### R6.3 PtyMode 输入

未被 Germinal 消费的 KeyEvent 在 PtyMode 下应发送给 PtyHost。

#### R6.4 GNativeAppMode 输入

未被 Germinal 消费的 KeyEvent 在 GNativeAppMode 下应发送给 GNativeApp。

#### R6.5 Command

系统需要提供统一 Command 模型，用于表达用户动作。

---

### R7. 渲染与帧合成

#### R7.1 PtyMode 渲染路径

```text
TerminalBuffer
-> TerminalRenderer
-> TerminalRenderBatch
-> PaneLayer
```

#### R7.2 GNativeAppMode 输出链路

```text
GNativeSDK / DSL
-> UiTree
-> AppLayout
-> RenderCommand
-> PaneLayer
```

#### R7.3 PaneLayer

每个 Pane 最终生成一个 PaneLayer。

#### R7.4 WindowFrame

Renderer 收集所有 PaneLayer，生成 WindowFrame。

#### R7.5 GPUFrame

Renderer 将 WindowFrame 绘制并提交为 GPUFrame。

---

### R8. 远程能力边界

#### R8.1 可序列化输入

KeyEvent、PointerEvent 和 Command 应尽量保持可序列化。

#### R8.2 可序列化输出

第一版不实现远程输出传输，但渲染输出边界应尽量保持可序列化。

PtyMode 侧预留 TerminalRenderBatch 的序列化边界。

GNativeAppMode 侧预留 RenderCommand 的序列化边界。

UiTree 属于 GNativeApp 内部结构，不作为远程协议边界。

#### R8.3 第一版非目标

第一版不实现远程运行、网络传输和多客户端同步。

## 4. 小需求列表

| ID   | 小需求                            | 优先级 |
| ---- | --------------------------------- | ------ |
| R1.1 | 创建 Window                       | P0     |
| R1.2 | 支持 Workspace                    | P1     |
| R1.3 | 支持 Tab                          | P1     |
| R1.4 | 支持 Pane                         | P0     |
| R1.5 | 支持 Pane 分屏和切换              | P0     |
| R2.1 | 为 Pane 创建 GShell               | P0     |
| R2.2 | 管理 GShell 生命周期              | P0     |
| R2.3 | GShell 默认启动 PtyHost           | P0     |
| R2.4 | GShell 默认进入 PtyMode           | P0     |
| R2.5 | 管理 GShellMode 生命周期          | P0     |
| R3.1 | 定义 PtyMode                      | P0     |
| R3.2 | 定义 GNativeAppMode               | P1     |
| R3.3 | 协议进入 GNativeAppMode           | P1     |
| R3.4 | 退出后返回 PtyMode                | P1     |
| R3.5 | 禁止智能识别模式                  | P0     |
| R4.1 | 创建 PTY                          | P0     |
| R4.2 | 启动 Shell                        | P0     |
| R4.3 | 维护 TerminalBuffer               | P0     |
| R4.4 | 转换终端输入                      | P0     |
| R4.5 | 生成 TerminalRenderBatch          | P0     |
| R5.1 | 管理 GNativeApp 生命周期          | P1     |
| R5.2 | 定义 GShellProtocol               | P1     |
| R5.3 | 提供 GNativeSDK / DSL             | P1     |
| R5.4 | 输出 UiTree                       | P1     |
| R5.5 | 定义 UiNode                       | P1     |
| R5.6 | 执行 AppLayout                    | P1     |
| R5.7 | 生成 RenderCommand                | P1     |
| R5.8 | 维护 UiFocus                      | P1     |
| R6.1 | 维护 ActiveGShell                 | P0     |
| R6.2 | 支持全局快捷键                    | P0     |
| R6.3 | PtyMode 输入进入 PtyHost          | P0     |
| R6.4 | GNativeAppMode 输入进入 GNativeApp | P1     |
| R6.5 | 定义 Command 模型                 | P1     |
| R7.1 | 实现 PtyMode 渲染路径             | P0     |
| R7.2 | 实现 GNativeAppMode 输出链路      | P1     |
| R7.3 | 生成 PaneLayer                    | P0     |
| R7.4 | 合成 WindowFrame                  | P0     |
| R7.5 | 提交 GPUFrame                     | P0     |
| R8.1 | 输入事件预留序列化边界            | P2     |
| R8.2 | 输出状态预留序列化边界            | P2     |
| R8.3 | 明确远程非目标                    | P0     |

## 5. 第一版最小闭环

第一版最小闭环应优先完成：

```text
Window
-> Pane
-> GShell
-> PtyHost
-> PtyMode
-> TerminalBuffer
-> TerminalRenderer
-> PaneLayer
-> WindowFrame
-> GPUFrame
```

该闭环完成后，Germinal 才具备传统终端兼容基础。

随后再实现：

```text
PtyHost 中运行 GNativeApp
-> GShellProtocol enter-native-app-mode
-> GNativeAppMode
-> GNativeSDK / DSL
-> UiTree
-> AppLayout
-> RenderCommand
-> PaneLayer
```

该闭环完成后，Germinal 才具备结构化应用平台基础。
