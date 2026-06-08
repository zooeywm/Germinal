use std::{env, mem::size_of, sync::Arc};

use germinal_ports::renderer::{Color as RenderColor, RenderCommand, RenderFrame, RendererPort};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap, fontdb,
};
use wgpu::util::DeviceExt;

const TEXT_SIZE: f32 = 15.0;
const LINE_HEIGHT: f32 = 16.0;

pub struct WgpuRendererBackend {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    rect_pipeline: wgpu::RenderPipeline,
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    terminal_font_family: String,
    rect_vertices: Vec<RectVertex>,
}

struct TextItem {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    font_size: f32,
    content: String,
    color: RenderColor,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RectVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl WgpuRendererBackend {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();

        let surface = instance
            .create_surface(window)
            .expect("failed to create wgpu surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("failed to request wgpu adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Germinal WGPU Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .expect("failed to request wgpu device");

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .expect("failed to create default surface config");

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Germinal Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Germinal Rect Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Germinal Rect Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_rect"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<RectVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: size_of::<[f32; 2]>() as u64,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_rect"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let mut font_system = FontSystem::new();
        let terminal_font_family = terminal_font_family(&font_system);
        font_system
            .db_mut()
            .set_monospace_family(terminal_font_family.clone());

        let swash_cache = SwashCache::new();
        let glyph_cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &glyph_cache);
        let mut text_atlas = TextAtlas::new(&device, &queue, &glyph_cache, config.format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            &device,
            wgpu::MultisampleState::default(),
            None,
        );

        Self {
            surface,
            device,
            queue,
            config,
            rect_pipeline,
            font_system,
            swash_cache,
            viewport,
            text_atlas,
            text_renderer,
            terminal_font_family,
            rect_vertices: Vec::new(),
        }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;

        self.surface.configure(&self.device, &self.config);
    }

    fn present_frame(&mut self, clear_color: RenderColor, text_items: &[TextItem]) {
        let surface_frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return;
            }
        };
        let text_buffers = self.prepare_text(text_items);
        let should_render_text = self.prepare_text_renderer(text_items, &text_buffers);
        let rect_vertex_buffer = if self.rect_vertices.is_empty() {
            None
        } else {
            Some(
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Germinal Rect Vertex Buffer"),
                        contents: rect_vertex_bytes(&self.rect_vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
            )
        };

        let surface_view = surface_frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Germinal Canvas Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Germinal Canvas Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(render_color_to_wgpu(clear_color)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            if let Some(buffer) = &rect_vertex_buffer {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..self.rect_vertices.len() as u32, 0..1);
            }

            if should_render_text
                && let Err(err) =
                    self.text_renderer
                        .render(&self.text_atlas, &self.viewport, &mut pass)
            {
                eprintln!("failed to render text: {err:?}");
            }
        }

        self.queue.submit(Some(encoder.finish()));
        surface_frame.present();
    }

    fn prepare_text(&mut self, text_items: &[TextItem]) -> Vec<Buffer> {
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        let mut buffers = Vec::with_capacity(text_items.len());

        for item in text_items {
            let line_height = terminal_line_height(item.font_size);
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(item.font_size, line_height),
            );
            buffer.set_monospace_width(
                &mut self.font_system,
                Some(terminal_cell_width(item.font_size)),
            );
            buffer.set_wrap(&mut self.font_system, Wrap::None);
            buffer.set_size(
                &mut self.font_system,
                Some(item.width.max(0.0)),
                Some(item.height.max(line_height)),
            );

            buffer.set_text(
                &mut self.font_system,
                &item.content,
                &Attrs::new().family(Family::Name(&self.terminal_font_family)),
                Shaping::Basic,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buffer);
        }

        buffers
    }

    fn prepare_text_renderer(&mut self, text_items: &[TextItem], text_buffers: &[Buffer]) -> bool {
        if text_buffers.is_empty() {
            return false;
        }

        let text_areas = text_items
            .iter()
            .zip(text_buffers.iter())
            .map(|(item, buffer)| TextArea {
                buffer,
                left: item.x,
                top: item.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: item.x.floor() as i32,
                    top: item.y.floor() as i32,
                    right: (item.x + item.width).ceil() as i32,
                    bottom: (item.y + item.height).ceil() as i32,
                },
                default_color: glyph_color(item.color),
                custom_glyphs: &[],
            });

        if let Err(err) = self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.text_atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        ) {
            eprintln!("failed to prepare text: {err:?}");
            return false;
        }

        true
    }

    fn prepare_frame(
        &mut self,
        frame: &RenderFrame,
        text_items: &mut Vec<TextItem>,
    ) -> RenderColor {
        let mut clear_color = RenderColor {
            r: 5,
            g: 5,
            b: 7,
            a: 255,
        };
        self.rect_vertices.clear();

        for command in frame.commands() {
            match command {
                RenderCommand::Clear(color) => {
                    clear_color = *color;
                }
                RenderCommand::FillRect {
                    x,
                    y,
                    width: rect_width,
                    height: rect_height,
                    color,
                } => {
                    push_rect_vertices(
                        &mut self.rect_vertices,
                        self.config.width as f32,
                        self.config.height as f32,
                        *x,
                        *y,
                        *rect_width,
                        *rect_height,
                        *color,
                    );
                }
                RenderCommand::Text {
                    x,
                    y,
                    width,
                    height,
                    font_size,
                    content,
                    color,
                } => {
                    text_items.push(TextItem {
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        font_size: *font_size,
                        content: content.clone(),
                        color: *color,
                    });
                }
            }
        }

        clear_color
    }
}

impl RendererPort for WgpuRendererBackend {
    fn render(&mut self, frame: &RenderFrame) {
        if self.config.width == 0 || self.config.height == 0 {
            return;
        }

        let mut text_items = Vec::new();
        let clear_color = self.prepare_frame(frame, &mut text_items);
        self.present_frame(clear_color, &text_items);
    }
}

fn glyph_color(color: RenderColor) -> GlyphColor {
    GlyphColor::rgba(color.r, color.g, color.b, color.a)
}

fn render_color_to_wgpu(color: RenderColor) -> wgpu::Color {
    wgpu::Color {
        r: color.r as f64 / 255.0,
        g: color.g as f64 / 255.0,
        b: color.b as f64 / 255.0,
        a: color.a as f64 / 255.0,
    }
}

fn push_rect_vertices(
    vertices: &mut Vec<RectVertex>,
    surface_width: f32,
    surface_height: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: RenderColor,
) {
    if surface_width <= 0.0 || surface_height <= 0.0 || width <= 0.0 || height <= 0.0 {
        return;
    }

    let left = clip_x(x, surface_width);
    let right = clip_x(x + width, surface_width);
    let top = clip_y(y, surface_height);
    let bottom = clip_y(y + height, surface_height);
    let color = [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ];

    vertices.extend_from_slice(&[
        RectVertex {
            position: [left, top],
            color,
        },
        RectVertex {
            position: [right, top],
            color,
        },
        RectVertex {
            position: [left, bottom],
            color,
        },
        RectVertex {
            position: [left, bottom],
            color,
        },
        RectVertex {
            position: [right, top],
            color,
        },
        RectVertex {
            position: [right, bottom],
            color,
        },
    ]);
}

fn clip_x(x: f32, surface_width: f32) -> f32 {
    ((x / surface_width) * 2.0 - 1.0).clamp(-1.0, 1.0)
}

fn clip_y(y: f32, surface_height: f32) -> f32 {
    (1.0 - (y / surface_height) * 2.0).clamp(-1.0, 1.0)
}

fn rect_vertex_bytes(vertices: &[RectVertex]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            vertices.as_ptr().cast::<u8>(),
            std::mem::size_of_val(vertices),
        )
    }
}

fn terminal_line_height(font_size: f32) -> f32 {
    (font_size * LINE_HEIGHT / TEXT_SIZE).max(1.0)
}

fn terminal_cell_width(font_size: f32) -> f32 {
    (font_size * 0.62).max(1.0)
}

fn terminal_font_family(font_system: &FontSystem) -> String {
    if let Ok(family) = env::var("GERMINAL_FONT_FAMILY")
        && has_font_family(font_system, &family)
    {
        return family;
    }

    const CANDIDATES: &[&str] = &[
        "JetBrainsMono Nerd Font",
        "JetBrainsMono Nerd Font Mono",
        "JetBrains Mono Nerd Font",
        "JetBrains Mono NL Nerd Font",
        "FiraCode Nerd Font",
        "FiraCode Nerd Font Mono",
        "Hack Nerd Font",
        "Hack Nerd Font Mono",
        "CaskaydiaCove Nerd Font",
        "CaskaydiaCove Nerd Font Mono",
        "MesloLGS NF",
        "MesloLGS Nerd Font",
        "JetBrains Mono",
    ];

    CANDIDATES
        .iter()
        .find(|family| has_font_family(font_system, family))
        .copied()
        .unwrap_or("monospace")
        .to_string()
}

fn has_font_family(font_system: &FontSystem, family: &str) -> bool {
    font_system
        .db()
        .query(&fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            ..fontdb::Query::default()
        })
        .is_some()
}

const RECT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_rect(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_rect(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;
