---
title: 前提背景技术调研与分析总结
---

## TUI: 各平台终端与 Shell 技术对比

目前市面上的终端在各平台上的现状如下：

| 平台         | 用户层通常叫什么                      | 底层机制                          | 典型应用                    |
| ------------ | ------------------------------------- | --------------------------------- | --------------------------- |
| Linux        | Terminal Emulator                     | POSIX PTY                         | Kitty, Wezterm, Alacritty   |
| macOS        | Terminal Emulator                     | BSD/POSIX PTY                     | iTerm2, Kitty, Alacritty    |
| Windows 传统 | Console / Terminal                    | Windows Console API + conhost.exe | cmd.exe, PowerShell         |
| Windows 现代 | Terminal Emulator / Terminal Frontend | ConPTY / Pseudo Console           | Windows Terminal, Alacritty |

Linux 和 macOS 的终端本质上很接近，底层创建 pseudo terminal，然后把 Shell 连接到这个 PTY 上。

Windows 历史上不是 Unix PTY 模型，而是 Console API 模型：

```
cmd.exe / powershell.exe -> Windows Console API -> conhost.exe -> console window
```

现代 Windows 引入了 ConPTY，全称是 Pseudo Console，作用类似 Unix PTY:

```
Windows Terminal / WezTerm -> ConPTY -> cmd.exe / PowerShell / bash.exe /ssh.exe
```

还有老一点的兼容方案是 `winpty`，比如早期 Git Bash、 MSYS2、 Cygwin 生态常用 winpty 解决 Windows 控制台兼容问题。  
所以对于现代而言，Linux、 macOS、 Windows上的终端可以都称为 Terminal Emulator，但由于 PTY 和 ConPTY 接口和协议的区别，这两者还是要区分的

Germinal 不打算兼容 Windows 传统 Console/Terminal，只提供 ConPTY 模式。

终端模拟器在

- POSIX 系统上，可从用户密码数据库解析默认 Shell，并根据终端配置以 login shell 或普通 shell 方式启动。

  创建一对 Unix PTY: Master/Slave，然后 Spawn Shell，将 Shell 的 stdin/stdout/stderr 连接到 PTY Slave，Terminal Emulator 自己持有 PTY Master，通过 Master 读写终端数据。

- Windows 系统上，通常根据配置使用 %COMSPEC% 或 cmd.exe 或 PowerShell。

  在 Windows 系统上创建 ConPTY，Spawn Shell，把 Shell 附着到这个 ConPTY，终端模拟器通过与 ConPTY 关联的 input/output pipe 读写数据。

## GUI: GPU 绘制合成和呈现的流程

| 阶段                             | 主要参与硬件     | 作用                                                                                                       |
| -------------------------------- | ---------------- | ---------------------------------------------------------------------------------------------------------- |
| 业务状态维护                     | CPU              | 维护应用业务状态，例如数据、选择、命令状态等                                                               |
| UI/Scene 构建                    | CPU              | 根据业务状态生成 UI 结构或场景结构，描述元素之间的层级、关系和状态                                         |
| 布局计算                         | CPU              | 确定元素位置、宽高、滚动显示区域                                                                           |
| 渲染命令生成                     | CPU              | 将布局后的 UI 或 Scene 转换为中间渲染命令                                                                  |
| 渲染资源准备                     | CPU + GPU        | 准备字体、图片、Glyph Atlas、Vertex Buffer、Texture 等渲染资源，并将必要数据上传到 GPU                     |
| 绘制                             | GPU              | 将渲染命令转换为 GPU Draw Call，把图形、文本、图片等绘制到渲染目标中                                       |
| 多个Layer Texture 组合成最终画面 | GPU              | 将图形的多层叠加为最终画面                                                                                 |
| 最终画面帧显示到屏幕             | GPU + 窗口管理器 | 合成结果写到 SwapChain 或 Surface Backbuffer 然后调用 present 将这一帧交给窗口系统或 Compositor 显示到屏幕 |

Texture 既可以作为输入资源，例如图片、字体 Glyph Atlas、视频帧；也可以作为输出目标，例如离屏渲染结果、Layer、中间缓存或后处理结果。最终画面也可能直接绘制到 swapchain 或 surface back buffer，而不一定必须先生成独立的 Texture。

在没有 GPU 或 GPU 不可用的情况下，系统通常可以使用软件渲染（Software Rendering）作为 fallback。此时渲染命令仍然可以复用，但绘制和 Layer 合成由 CPU 完成，输出目标通常是 CPU bitmap 或 software surface，最后再通过平台窗口系统提交显示。

对于 Germinal 这类结构化 UI 平台，还需要在“作者接口”和“GPU 后端”之间明确插入一条内部链路。应用作者通常先通过 DSL 或 SDK 描述 UI，再由运行时逐步降低为结构化 IR 和渲染 IR：

```text
DSL / GNativeSDK
-> UiTree
-> AppLayout
-> RenderCommand
-> Batching
-> Draw Call
-> GPU
```

其中 `UiTree` 表达结构与语义，`RenderCommand` 表达高层绘制语义边界，二者都不等同于底层 GPU 命令。

### 渲染资源准备：

| 资源           | 作用                                      |
| -------------- | ----------------------------------------- |
| Vertex Buffer  | 描述图形顶点                              |
| Index Buffer   | 描述顶点复用关系                          |
| Texture        | 图片、字形缓存、视频帧、离屏渲染结果      |
| Glyph Atlas    | 把很多字体字形缓存到一张或多张纹理里      |
| Uniform Buffer | 当前窗口大小、变换矩阵、颜色参数等        |
| Pipeline       | GPU 绘制规则，例如用哪个 shader、如何混合 |

### GPU 绘制

GPU 绘制大概链路：

```text
RenderCommand -> Batching -> Draw Call -> Vertex Shader -> Rasterization -> Fragment Shader -> Framebuffer / Texture
```

其中 RenderCommand，Batching 是在 CPU，Draw Call 是 CPU 提交到 GPU。

| 阶段            | 作用                                       | 举例                                           |
| --------------- | ------------------------------------------ | ---------------------------------------------- |
| RenderCommand   | 记录所有渲染命令                           | 绘制文字、矩形、图片、背景、边框等的一组命令   |
| Batching        | 合并可以一起绘制的命令，减少Draw Call 数量 | 相同字体、相同材质、相同纹理图集的文字合并绘制 |
| Draw Call       | 通知GPU执行一次绘制                        | 绘制一个矩形                                   |
| Vertex Shader   | 处理几何位置                               | 比如一个矩形会变成屏幕上几个三角形             |
| Rasterization   | 把几何图形转换成像素覆盖区域               | 这个三角形覆盖了哪些像素                       |
| Fragment Shader | 决定每个像素是什么颜色                     | 文字颜色、背景颜色、透明混合后颜色             |
| Texture         | 渲染资源或结果                             | 作为输入资源或输出结果                         |

## 传统 TUI 与 GUI 程序的能力对比

### 使用效率

开发者高频、批量操作：TUI 更高效。

复杂可视化/多媒体/拖拽任务：GUI 更高效。

### 计算

传统 TUI 的复杂度集中在终端语义解析、字符网格、宽字符处理和 Screen Buffer Diff；GUI 的复杂度集中在场景树、布局、文本 shaping、裁剪和合成。二者复杂点不同，不能简单判断谁一定更低。

对比

| 阶段       | 传统 TUI/终端链路                                               | GUI/RenderCommand 链路                                            |
| ---------- | --------------------------------------------------------------- | ----------------------------------------------------------------- |
| 布局目标   | 计算每个字符单元格放什么                                        | 计算每个图形元素位置、尺寸、层级                                  |
| 输出单位   | Cell/Grapheme/字符属性                                          | 矩形、文字、图片、裁剪区等绘制命令                                |
| 坐标系统   | 行列网格                                                        | 像素/逻辑坐标                                                     |
| 宽度处理   | 很复杂                                                          | 不依赖 Cell 网格，但仍涉及文本测量、换行和 shaping。              |
| 文本处理   | 极复杂                                                          | 复杂度主要转移到 shaping、字体 fallback、换行、裁剪和 glyph cache |
| 局部更新   | 通常依赖 Screen Buffer Diff / Damage Tracking                   | 可直接重绘局部区域或整帧重建命令                                  |
| 样式叠加   | 前景色、背景色、bold、underline、reverse、selection 等压到 Cell | 作为绘制属性存在                                                  |
| 滚动处理   | 涉及历史缓冲区、View Port、软换行、行重排                       | 裁剪 + Transform                                                  |
| 光标处理   | 必须映射到具体Cell                                              | 是一个普通绘制对象                                                |
| 复杂度核心 | 文本网格语义                                                    | 结构化布局与图形合成语义                                          |

### 布局灵活度

GUI 是像素/矢量二维布局，能任意定位、缩放、层叠、滚动、约束布局。

TUI 是字符网格布局，只能按行列 Cell 排布，布局自由度低。

### 渲染硬件加速能力

现代终端模拟器可以用 GPU 加速 TUI 的字符光栅化、字形缓存、颜色填充、滚动、合成、局部刷新。

但与 GUI 的布局表达能力差距仍明显；性能是否更低取决于场景。关键原因是TUI 加速的是“字符网格渲染”，GUI 加速的是“任意图形场景渲染”。

硬件加速能缩小性能差距，但不能消除表达和布局差距。

## 分析与总结

可以看出

1. TUI 程序通过熟练使用快捷键，开发操作、 批量操作更高效；GUI 程序复杂可视化/多媒体/拖拽任务更高效
2. 传统 TUI 的计算复杂度集中在字符网格和终端语义，GUI 的复杂度集中在结构化布局和图形合成。
3. TUI 渲染不一定比 GUI 慢，但受 Cell 网格限制，布局和表达能力明显弱于 GUI。

传统终端技术能提供高效键盘操作和文本工作流，但受 Cell 网格限制，难以获得 GUI 级布局表达能力。Germinal 的价值在于把 TUI 的键盘效率与 GUI 的结构化布局、GPU 渲染和应用平台能力结合起来。

### 目前有 GUI 单应用以 TUI 操作模式的应用

比如 Zed、Neovide 等，但虽然一个 GUI 程序是可以自己参照 TUI 风格去实现类似 TUI 的快捷键操作，但不同作者制作的不同类似应用之间无法形成统一的操作平台

| 对比     | Germinal                                          | GUI App + TUI 操作    |
| -------- | ------------------------------------------------- | --------------------- |
| 作用范围 | 多 GShell / 多 Mode / 多 Pane                     | 单 App 内部           |
| 复用性   | 高，输入、布局、渲染和协议能力可给 GNativeApp 使用 | 低，每个 App 自己实现 |
| 组合能力 | 强，Pane / GShell 可组合, GShellMode 可切换       | 弱，受 App 边界限制   |
| 生态能力 | 高，可形成生态                                    | 局部优化              |
