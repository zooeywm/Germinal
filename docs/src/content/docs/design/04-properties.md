---
title: Germinal 性质推导
---

## 1. 目标

本文从 `requirements-breakdown.md` 中的小需求出发，推导 Germinal 第一版必须满足的核心工程性质。

这些性质用于约束后续 DDD 建模和技术方案设计。

## 2. 核心性质

### P1. 层级结构必须稳定

Germinal 的基础结构必须保持清晰稳定：

```text
Window
-> Workspace
-> Tab
-> Pane
-> GShell
-> GShellMode
```

该结构不能被 PtyHost 或 GNativeApp 反向污染。

### P2. Pane 与 GShell 必须一一对应

一个 Pane 只能承载一个 GShell。

多个 GShell 通过多个 Pane 组织。

这样可以保证布局、焦点、生命周期和渲染边界清晰。

### P3. GShell 必须默认启动 PtyHost

一个 GShell 创建后必须默认启动 PtyHost。

PtyHost 是 GShell 的基础兼容组件，不是临时可选能力。

默认使用路径是：

```text
GShell
-> PtyHost
-> PtyMode
```

### P4. GShellMode 必须由协议显式切换

GShellMode 可以是：

```text
PtyMode | GNativeAppMode
```

同一时刻一个 GShell 只能处于一种模式。

进入 GNativeAppMode 必须由 GNativeProtocol 显式触发。

退出 GNativeAppMode 后必须返回 PtyMode。

系统不做智能识别。

### P5. PtyHost 与 GNativeApp 的状态必须分离

PtyHost 自己管理：

```text
PTY
TerminalBuffer
TerminalCursor
Scrollback
Selection
```

GNativeApp 自己管理：

```text
AppState
UiTree
UiFocus
```

GNativeSDK / DSL 属于 GNativeApp 作者侧接口，不应污染 GShell 或 PtyHost 的运行时状态模型。

GShell 只负责输入分发、模式切换、生命周期和渲染桥接。

### P6. 输入必须先经过 Germinal，再进入 ActiveGShell

KeyEvent / PointerEvent 的处理顺序必须是：

```text
Input Event
-> Germinal
-> ActiveGShell
-> Current GShellMode
```

Germinal 优先处理全局命令，例如 Pane 切换、Tab 切换、Workspace 切换。

未被 Germinal 消费的事件再根据当前 GShellMode 分发：

```text
PtyMode         -> PtyHost
GNativeAppMode  -> GNativeApp
```

### P7. 键盘必须是一等输入

Germinal 的核心交互必须能通过键盘完成。

鼠标和触控板只能作为辅助输入。

第一版至少要保证以下能力可通过键盘完成：

```text
切换 Pane
切换 Tab
切换 Workspace
触发 Command
向 PtyHost 输入文本或快捷键
向 GNativeApp 发送应用级快捷键
```

### P8. PtyMode 与 GNativeAppMode 必须使用不同渲染路径

PtyMode 渲染路径：

```text
TerminalBuffer
-> TerminalRenderer
-> TerminalRenderBatch
-> PaneLayer
```

GNativeAppMode 渲染路径：

```text
GNativeSDK / DSL
-> UiTree
-> AppLayout
-> RenderCommand
-> PaneLayer
```

二者不能混成一条路径。

### P9. Renderer 必须只消费渲染结果

Renderer 不应理解 PtyHost 或 GNativeApp 的业务逻辑。

Renderer 只消费：

```text
TerminalRenderBatch
RenderCommand
PaneLayer
WindowFrame
```

这样可以保证渲染层和应用层解耦。

### P10. PaneLayer 必须是帧合成边界

每个 Pane 最终生成一个 PaneLayer。

WindowFrame 由多个 PaneLayer 合成。

```text
PaneLayer #1
PaneLayer #2
-> WindowFrame
-> GPUFrame
```

### P11. UiTree 不能作为远程协议边界

UiTree 是 GNativeApp 内部结构。

未来远程能力不应直接传输 UiTree。

远程输出边界应预留为：

```text
PtyMode         -> TerminalRenderBatch
GNativeAppMode  -> RenderCommand
```

### P12. RenderCommand 必须是高层绘制语义

RenderCommand 应表达平台无关的 2D UI 绘制语义，例如：

```text
DrawText
DrawRect
DrawImage
DrawVideoFrame
Clip
Transform
Opacity
Layer
```

RenderCommand 对接的是 GShell renderer/compositor，不应暴露底层 GPU 对象，例如：

```text
wgpu CommandBuffer
Texture Handle
Buffer Handle
Pipeline
BindGroup
Shader Object
```

### P13. 第一版必须先完成 PtyMode 闭环

第一版最小闭环应优先保证：

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

只有该闭环完成后，Germinal 才具备基础终端能力。

### P14. GNativeAppMode 是第二个闭环

结构化应用闭环为：

```text
PtyHost 中运行 GNativeApp
-> GNativeProtocol enter-native-app-mode
-> GNativeAppMode
-> GNativeSDK / DSL
-> UiTree
-> AppLayout
-> RenderCommand
-> PaneLayer
```

该闭环完成后，Germinal 才具备结构化 UI 平台能力。

## 3. 总结

Germinal 第一版的核心工程性质是：

```text
层级稳定
Pane 与 GShell 一对一
GShell 默认 PtyHost
协议显式切换 GShellMode
不做智能识别
PtyHost 与 GNativeApp 状态分离
键盘优先
输入统一分发
渲染路径分离
PaneLayer 合成
RenderCommand 平台无关
远程边界不暴露 UiTree
```
