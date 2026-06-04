---
title: Germinal 核心问题定义与设计目标
---

Germinal，全称是 Graphical Terminal，是在传统终端交互范式基础上，同时提供完整 PTY 运行时能力与结构化 UI 能力的键盘优先、命令驱动图形化终端系统。

Germinal 默认以传统终端方式启动，通过 PtyHost 运行 Shell、CLI 和传统 TUI 程序。当用户运行支持 Germinal 协议的 GNativeApp 时，GShell 通过显式协议进入 GNativeAppMode；GNativeApp 退出后，GShell 返回 PtyMode。

## 1. 背景结论

传统 TUI 建立在终端模拟器、PTY/ConPTY、Shell 和字符网格之上，优势是键盘操作高效、文本工作流成熟、远程和批量操作友好。但它的 UI 表达最终必须落到 Cell 网格中，因此在布局自由度、图形表达、层叠合成和复杂交互上存在天然限制。

GUI 程序建立在结构化 UI、布局系统、RenderCommand、GPU 绘制和窗口系统合成之上，优势是布局自由、表达能力强，适合复杂可视化、多媒体、动画和精细交互。但普通 GUI 应用通常是单应用封闭体系，难以形成统一的键盘优先操作平台。

现代终端模拟器已经可以利用 GPU 加速字符渲染、滚动、合成和局部刷新，但它加速的仍然是字符网格渲染，不能从根本上突破 TUI 的 Cell 网格约束。

因此，Germinal 的核心机会不是做一个更快的传统终端，而是在传统终端工作流中引入结构化 UI 模式，构建一个键盘优先、结构化布局、GPU 渲染、可承载多应用生态的开发者 UI 平台。

## 2. 核心问题

传统 TUI 的核心问题不是性能不足，而是 UI 表达模型受 Cell 网格限制。即使终端模拟器使用 GPU 加速，TUI 应用最终仍然只能输出字符、颜色和少量样式，难以表达自由布局、图形层叠、多媒体内容和复杂交互。

GUI 的核心问题不是表达能力不足，而是缺少统一的键盘优先应用平台。单个 GUI 应用可以模拟 TUI 操作方式，例如 Vim 模式、快捷键驱动和命令面板，但这些能力通常只存在于应用内部，无法跨应用复用，也难以形成统一的工作流组合能力。

因此，Germinal 要解决的核心问题是：

如何在保留 TUI 键盘效率和开发者工作流优势的同时，允许支持 Germinal 协议的应用从传统 PtyMode 显式进入 GNativeAppMode，从而突破传统终端 Cell 网格限制，提供结构化布局、GPU 渲染、可组合、多应用共享的开发者 UI 平台。

## 3. 设计目标

Germinal 的设计目标是构建一个键盘优先、结构化布局、GPU 渲染的开发者 UI 平台。

具体目标如下：

1. 保留 TUI 的高效键盘操作能力，使开发者可以通过快捷键、命令和组合操作完成高频工作流。
2. 默认通过 PtyHost 运行传统 Shell、CLI 和 TUI 程序，使用户可以从普通 shell 工作流自然进入 Germinal。
3. 通过 GNativeProtocol 支持从 PtyMode 显式进入 GNativeAppMode，而不是依赖智能识别。
4. 为 GNativeApp 提供 GNativeSDK / DSL 这一层作者接口，而不是要求应用作者直接手写 UiTree 或 RenderCommand。
5. 通过 `DSL -> UiTree -> AppLayout -> RenderCommand` 链路突破传统终端 Cell 网格限制，表达结构化 UI。
6. 使用 GPU 渲染作为主要绘制路径，支持文字、图形、图片、视频和复杂 UI 的统一合成。
7. 为未来远程能力预留协议边界，但第一版不实现远程运行、多客户端同步和网络传输。

## 4. 非目标

Germinal 不以替代所有 GUI 应用为目标，也不追求成为通用桌面应用框架。

Germinal 不通过智能识别判断一个进程是否是 GNativeApp。GNativeAppMode 的进入和退出必须由明确的 GNativeProtocol 触发。

Germinal 第一版需要具备完整的 PtyHost / PtyMode 运行时能力，使传统 Shell、CLI 和 TUI 程序能够作为默认工作流稳定运行；GNativeAppMode 是在这一基础上的显式运行时切换，而不是对部分终端兼容能力的替代。

Germinal 第一版不以鼠标优先交互为目标。鼠标可以作为辅助输入，但键盘必须是一等公民。

Germinal 第一版不要求应用作者直接编写底层 GPU 命令，也不要求应用作者手写 RenderCommand。

Germinal 第一版不实现远程运行、多客户端同步、网络传输和远程 GPU 渲染，只在架构上预留未来远程能力边界。

## 5. Germinal 的定位

Germinal 不是传统终端模拟器的简单增强版，也不是普通 GUI 应用框架。

Germinal 的定位是：面向开发者工作流的键盘优先结构化 UI 平台。

它同时承担两类能力：

1. 作为完整 PTY 运行时，通过 PtyHost 承载 Shell、CLI 和现有 TUI 程序。
2. 作为结构化应用平台，通过 GNativeProtocol 让支持 Germinal 的应用进入 GNativeAppMode，承载具备自由布局、GPU 渲染和统一交互模型的新型开发者应用。

因此，Germinal 的核心价值不是“让终端更快”，而是提供一种从传统终端自然演进到结构化 UI 的新平台：保留 TUI 的键盘效率和组合习惯，同时获得 GUI 的布局表达能力和渲染能力。

## 6. 开发与渲染主链路

GNativeApp 从开发到最终上屏的主链路应明确区分作者接口、内部 IR、协议边界和 GPU 后端：

```text
GNativeApp Source
-> GNativeSDK / DSL
-> UiTree
-> AppLayout
-> RenderCommand
-> GShell renderer/compositor
-> GPU backend
```

其中：

- `GNativeSDK / DSL` 是面向应用作者的高层声明式接口。
- `UiTree` 是结构化 UI 内部 IR，用于表达节点层级、语义和属性。
- `AppLayout` 负责把结构化节点计算为几何结果。
- `RenderCommand` 是 GNativeApp 输出给 GShell renderer/compositor 的高层绘制语义边界，不是底层 GPU 命令。

## 7. 成功标准

Germinal 第一版成功的标准是：

1. 可以稳定运行传统 Shell、CLI 和常见 TUI 程序，并通过 PtyHost 提供完整 PTY 运行时能力。
2. 每个 Pane 可以稳定承载一个 GShell，GShell 默认进入 PtyMode。
3. 可以通过明确的 GNativeProtocol 从 PtyMode 进入 GNativeAppMode，并在 GNativeApp 退出后返回 PtyMode。
4. 可以承载至少一个 GNativeApp，使其开发入口是 GNativeSDK / DSL，而不是字符网格。
5. 可以把 GNativeApp 的 `UiTree -> AppLayout -> RenderCommand` 链路稳定转换为宿主侧渲染输出。
6. 可以在同一窗口内统一管理多个 Workspace、Tab、Pane 和不同 Mode 的 GShell。
7. 可以提供统一的键盘优先交互模型，包括 ActiveGShell 切换、Pane 操作、命令触发和应用级快捷键。
8. 架构边界清晰，第一版不实现远程能力，但不会阻碍未来扩展远程协议、远程事件和远程状态同步。
