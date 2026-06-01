---
title: 工程分层与依赖规则
---

本文定义 Germinal 的工程分层、crate 依赖方向与各层职责边界。

本文主要描述 GShell 宿主侧工程分层。`GNativeSDK / DSL -> UiTree -> AppLayout -> RenderCommand` 这条作者侧链路与宿主分层并行存在，但不应反向污染宿主 `domain / application / ports / infra / app` 的依赖规则。

`ports` 不是独立先验层，而是由 `application` 的外部能力需求驱动形成的抽象边界：

```txt
domain
↓
application
↓
ports
↓
infra
↓
app
```

设计顺序不影响代码依赖方向，代码依赖始终保持单向。

## 总体原则

Germinal 采用严格单向依赖：

```txt
app
 ├── application
 │    ├── domain
 │    └── ports
 └── infra
      └── ports
```

依赖关系：

```txt
app -> application
app -> infra

application -> domain
application -> ports

infra -> ports

domain -> nothing
ports  -> nothing
```

禁止反向依赖。

设计顺序：

```txt
domain
→ application
→ ports
→ infra
→ app
```

代码依赖：

```txt
application -> ports
infra -> ports
```

即：

```txt
application 定义需求
ports 定义接口
infra 提供实现
```

## 分层职责

### domain

`domain` 是纯领域模型层。

负责表达核心业务概念、规则与状态变化，不关心运行环境与外部技术实现。

可包含：

```txt
Entity
Value Object
Aggregate
领域服务
领域事件
领域规则
领域状态转换
```

禁止依赖：

```txt
application
ports
infra
app
wgpu
compio
serde_json 等协议实现
操作系统 API
渲染 API
窗口系统 API
```

允许使用稳定基础库：

```txt
std
thiserror
uuid
slotmap
smallvec
```

避免基础设施语义进入领域模型。

## application

`application` 是应用流程编排层。

负责协调领域对象完成业务流程，并定义系统需要的外部能力。

能力通过 `ports` 声明，不在本层实现。

可依赖：

```txt
domain
ports
```

可包含：

```txt
Manager
Coordinator
Controller
Command
Query
Application Service
事件分发
事务边界
DTO 映射
流程编排
外部能力需求
```

典型职责：

```txt
创建窗口
创建 Workspace
创建 Tab
创建 Pane
启动 PtySession
处理输入事件
驱动领域状态变化
生成 RenderFrameDto
调用 RendererPort
```

示例：

```rust
pub struct WorkspaceManager<R, P>
where
    R: RendererPort,
    P: PtyPort,
{
    renderer: R,
    pty: P,
    workspace: Workspace,
}

impl<R, P> WorkspaceManager<R, P>
where
    R: RendererPort,
    P: PtyPort,
{
    pub fn open_pty_pane(&mut self, config: OpenPaneCommand) -> Result<(), OpenPaneError> {
        let pty = self.pty.spawn(config.into_pty_config())?;

        self.workspace.open_pane(pty.into_session_id())?;

        let frame = RenderFrameDto::from_workspace(&self.workspace);

        self.renderer.submit_frame(frame)?;

        Ok(())
    }
}
```

这里 `WorkspaceManager` 负责流程协调，不维护领域规则。

禁止直接依赖：

```txt
wgpu
vulkan
wayland-client
x11rb
compio
文件系统实现
网络实现
窗口系统实现
```

这些能力属于 `infra`。

## ports

`ports` 是外部能力抽象层。

负责定义 `application` 所需能力的接口，不提供实现。

可包含：

```txt
trait
DTO
Handle
Config
Error
边界类型
```

示例：

```rust
pub trait RendererPort {
    fn submit_frame(&mut self, frame: RenderFrameDto) -> Result<(), RendererError>;
}

pub trait WindowPort {
    fn request_redraw(&mut self, window: WindowHandle);
}

pub trait PtyPort {
    fn spawn(&mut self, config: PtySpawnConfig) -> Result<PtyHandle, PtyError>;
}
```

禁止使用 `domain` 类型作为接口参数。

错误示例：

```rust
use germinal_domain::Workspace;

pub trait RendererPort {
    fn render_workspace(&mut self, workspace: &Workspace);
}
```

正确做法：

```rust
pub struct RenderFrameDto {
    pub frame_id: u64,
    pub surfaces: Vec<SurfaceDto>,
    pub commands: Vec<RenderCommandDto>,
}

pub trait RendererPort {
    fn submit_frame(&mut self, frame: RenderFrameDto) -> Result<(), RendererError>;
}
```

由 `application` 将领域状态转换为 DTO。

禁止依赖：

```txt
domain
application
infra
app
```

## infra

`infra` 是基础设施实现层。

负责实现 `ports` 中定义的接口，并封装平台能力与第三方库。

可依赖：

```txt
ports
wgpu
winit
compio
nix
wayland-client
windows
serde_json
其他外部库
```

禁止依赖：

```txt
domain
application
```

错误示例：

```rust
use germinal_domain::Workspace;
use germinal_ports::RendererPort;

pub struct WgpuRenderer;

impl RendererPort for WgpuRenderer {
    fn render_workspace(&mut self, workspace: &Workspace) {
        // ...
    }
}
```

正确示例：

```rust
use germinal_ports::{RendererPort, RenderFrameDto, RendererError};

pub struct WgpuRenderer;

impl RendererPort for WgpuRenderer {
    fn submit_frame(&mut self, frame: RenderFrameDto) -> Result<(), RendererError> {
        // 使用 wgpu 渲染 RenderFrameDto
        Ok(())
    }
}
```

异步能力（PTY、文件 IO、网络 IO、事件驱动）同样通过 `ports` 抽象，不向上暴露运行时类型。

## app

`app` 是最终组装层。

负责组合 `application` 与 `infra`，形成可运行程序。

可依赖：

```txt
application
infra
ports
```

可包含：

```txt
main.rs
依赖注入
运行时初始化
日志初始化
配置加载
平台选择
基础设施绑定
事件循环启动
```

示例：

```rust
fn main() -> anyhow::Result<()> {
    let renderer = WgpuRenderer::new()?;
    let pty = SystemPty::new()?;
    let window = WinitWindowSystem::new()?;

    let app = GerminalApp::new(renderer, pty, window);

    app.run()?;

    Ok(())
}
```

运行时生命周期管理放在这一层。

## 补充：GNativeApp 作者层

GNativeApp 作者侧还需要一条独立的结构化 UI authoring 链路：

```txt
GNativeSDK / DSL
-> UiTree
-> AppLayout
-> RenderCommand
-> GShellProtocol / transport
```

这条链路的职责分工是：

- `GNativeSDK / DSL`：面向应用作者的高层声明式接口。
- `UiTree`：结构化 UI 内部 IR，表达节点层级、语义和属性。
- `AppLayout`：把结构化节点计算成几何结果。
- `RenderCommand`：发送给 GShell renderer/compositor 的高层绘制语义边界。

它不是宿主五层架构里的额外一层，而是与宿主并行的 app authoring/runtime 体系。

### 推荐 transport 分层

本地模式下，`RenderCommand` 与资源不应共用一条“全塞进 PTY”的链路。推荐拆成：

```txt
PTY
  -> 启动 shell
  -> 启动 GNativeApp
  -> enter-native-app-mode / exit-native-app-mode 握手

Command transport
  -> RenderCommand frame
  -> 小型 resource metadata

Resource transport
  -> 图片像素
  -> 视频帧
  -> 离屏 surface
```

推荐实现顺序：

1. 本地 MVP：`Unix domain socket` 传 RenderCommand 和小型资源消息。
2. 后续增强：大资源改为 `shared memory / memfd`，命令里只保留 `resource_id` 引用。
3. 远程扩展：延续同样边界，把 `RenderCommand frame` 和资源更新映射到远程 transport。

这样可以保证：

- PTY 继续只负责传统终端兼容和模式切换入口。
- RenderCommand 成为清晰的 renderer/compositor 输入边界。
- 纹理、视频帧等大对象不被迫重复序列化进每一帧命令里。

## 推荐 workspace 结构

```txt
crates/
├── germinal-domain/
├── germinal-application/
├── germinal-ports/
├── gnative-ui/
├── gnative-protocol/
├── gnative-sdk/
├── germinal-infra/
│   ├── germinal-infra-renderer-wgpu/
│   ├── germinal-infra-window-winit/
│   ├── germinal-infra-pty-unix/
│   └── germinal-infra-storage/
└── germinal-app/
```

依赖关系：

```txt
germinal-domain
  -> no internal dependency

germinal-application
  -> germinal-domain
  -> germinal-ports

germinal-ports
  -> no internal dependency

gnative-ui
  -> no internal dependency

gnative-protocol
  -> no internal dependency

gnative-sdk
  -> gnative-ui
  -> gnative-protocol

germinal-infra-*
  -> germinal-ports

germinal-app
  -> germinal-application
  -> germinal-infra-*

GNativeApp binaries
  -> gnative-sdk
  -> gnative-ui
  -> gnative-protocol
```

## 规则总结

| 层          | 职责         | 可依赖                      | 禁止依赖                           |
| ----------- | ------------ | --------------------------- | ---------------------------------- |
| domain      | 领域模型     | 基础库                      | application / ports / infra / app  |
| application | 流程编排     | domain / ports              | infra / app                        |
| ports       | 外部能力抽象 | 基础库                      | domain / application / infra / app |
| infra       | 能力实现     | ports / 外部库              | domain / application               |
| app         | 最终组装     | application / infra / ports | 无                                 |

`gnative-*` crate 不属于宿主五层之一。它们属于 GNativeApp 作者侧 SDK/runtime，应通过 `RenderCommand` 和协议边界与宿主衔接，而不是直接依赖宿主 `application` 或 `infra` 内部实现。

## 核心判断标准

```txt
表达业务概念
→ domain

协调流程与行为
→ application

定义外部能力接口
→ ports

调用系统 API 或第三方库
→ infra

负责程序组装
→ app
```

## 核心约束

```txt
domain 保持纯净

application 定义能力需求

ports 不暴露 domain

application 负责 DTO 转换

infra 只实现 ports

app 不承载业务规则
```

补充约束：

```txt
GNativeSDK 不直接暴露 wgpu / winit

UiTree 不是宿主远程协议边界

RenderCommand 是 GNativeApp 到 GShell renderer/compositor 的边界
```
