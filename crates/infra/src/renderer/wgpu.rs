use std::{env, mem::size_of, sync::Arc};

use germinal_ports::renderer::{Color as RenderColor, RenderCommand, RenderFrame, RendererPort};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Wrap, fontdb,
};

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
    rect_vertices: Vec<RectVertex>,
    rect_vertex_buffer: Option<wgpu::Buffer>,
    rect_vertex_capacity: usize,

    text_cache: Vec<CachedTextItem>,
    cjk_font_family: Option<String>,
    symbol_font_family: Option<String>,
    complex_font_family: Option<String>,
}

struct CachedTextItem {
    key: TextCacheKey,
    buffer: Buffer,
}

#[derive(PartialEq, Eq)]
struct TextCacheKey {
    width: u32,
    height: u32,
    font_size: u32,
    style: TextStyle,
    content: String,
}

impl TextCacheKey {
    fn from_item(item: &TextItem) -> Self {
        Self {
            width: item.width.to_bits(),
            height: item.height.to_bits(),
            font_size: item.font_size.to_bits(),
            style: item.style,
            content: item.content.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextStyle {
    MonospaceBasic,
    CjkBasic,
    SymbolBasic,
    Advanced,
}

struct TextItem {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    font_size: f32,
    style: TextStyle,
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
        let cjk_font_family = first_available_font_family(
            &font_system,
            &[
                "Noto Sans Mono CJK SC",
                "Noto Sans CJK SC",
                "Noto Sans Mono CJK JP",
                "Noto Sans CJK JP",
            ],
        );
        let symbol_font_family = first_available_font_family(
            &font_system,
            &[
                "Noto Sans Symbols2",
                "Noto Sans Symbols 2",
                "Noto Sans Symbols",
            ],
        );
        let complex_font_family = first_available_font_family(&font_system, &["Noto Sans"]);
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
            rect_vertices: Vec::new(),
            rect_vertex_buffer: None,
            rect_vertex_capacity: 0,
            text_cache: Vec::new(),
            cjk_font_family,
            symbol_font_family,
            complex_font_family,
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
        self.prepare_text_cache(text_items);
        let should_render_text = self.prepare_text_renderer(text_items);
        self.update_rect_vertex_buffer();

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

            if let Some(buffer) = &self.rect_vertex_buffer {
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

    fn update_rect_vertex_buffer(&mut self) {
        if self.rect_vertices.is_empty() {
            return;
        }

        if self.rect_vertices.len() > self.rect_vertex_capacity {
            self.rect_vertex_capacity = self.rect_vertices.len().next_power_of_two();
            self.rect_vertex_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Germinal Rect Vertex Buffer"),
                size: (self.rect_vertex_capacity * size_of::<RectVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }

        if let Some(buffer) = &self.rect_vertex_buffer {
            self.queue
                .write_buffer(buffer, 0, rect_vertex_bytes(&self.rect_vertices));
        }
    }

    fn prepare_text_cache(&mut self, text_items: &[TextItem]) {
        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        let mut old_cache = std::mem::take(&mut self.text_cache);
        let mut new_cache = Vec::with_capacity(text_items.len());

        for item in text_items {
            let key = TextCacheKey::from_item(item);

            if let Some(index) = old_cache.iter().position(|cached| cached.key == key) {
                new_cache.push(old_cache.swap_remove(index));
            } else {
                let buffer = self.create_text_buffer(item);
                new_cache.push(CachedTextItem { key, buffer });
            }
        }

        self.text_cache = new_cache;
    }

    fn create_text_buffer(&mut self, item: &TextItem) -> Buffer {
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

        match item.style {
            TextStyle::CjkBasic => {
                if let Some(family) = &self.cjk_font_family {
                    buffer.set_text(
                        &mut self.font_system,
                        &item.content,
                        &Attrs::new().family(Family::Name(family)),
                        Shaping::Basic,
                        None,
                    );
                } else {
                    self.set_advanced_text(&mut buffer, item);
                }
            }
            TextStyle::SymbolBasic => {
                if let Some(family) = &self.symbol_font_family {
                    buffer.set_text(
                        &mut self.font_system,
                        &item.content,
                        &Attrs::new().family(Family::Name(family)),
                        Shaping::Basic,
                        None,
                    );
                } else {
                    self.set_advanced_text(&mut buffer, item);
                }
            }
            TextStyle::Advanced => self.set_advanced_text(&mut buffer, item),
            TextStyle::MonospaceBasic => {
                buffer.set_text(
                    &mut self.font_system,
                    &item.content,
                    &Attrs::new().family(Family::Monospace),
                    Shaping::Basic,
                    None,
                );
            }
        }

        buffer.shape_until_scroll(&mut self.font_system, false);

        buffer
    }

    fn set_advanced_text(&mut self, buffer: &mut Buffer, item: &TextItem) {
        if let Some(family) = &self.complex_font_family {
            buffer.set_text(
                &mut self.font_system,
                &item.content,
                &Attrs::new().family(Family::Name(family)),
                Shaping::Advanced,
                None,
            );
        } else {
            buffer.set_text(
                &mut self.font_system,
                &item.content,
                &Attrs::new().family(Family::Monospace),
                Shaping::Advanced,
                None,
            );
        }
    }

    fn prepare_text_renderer(&mut self, text_items: &[TextItem]) -> bool {
        if self.text_cache.is_empty() {
            return false;
        }

        let text_areas = text_items
            .iter()
            .zip(self.text_cache.iter())
            .map(|(item, cached)| TextArea {
                buffer: &cached.buffer,
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
                    push_text_items(
                        text_items, *x, *y, *width, *height, *font_size, content, *color,
                    );
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

fn push_text_items(
    text_items: &mut Vec<TextItem>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    font_size: f32,
    content: &str,
    color: RenderColor,
) {
    let cell_width = terminal_cell_width(font_size);
    let mut run_style = None;
    let mut run_content = String::new();
    let mut run_x = x;
    let mut run_width = 0.0;
    let mut cursor_x = x;

    for ch in content.chars() {
        let style = text_style(ch);
        let ch_width = char_cell_width(ch) as f32 * cell_width;

        if run_style.is_some_and(|current| current != style) {
            text_items.push(TextItem {
                x: run_x,
                y,
                width: run_width,
                height,
                font_size,
                style: run_style.expect("run_style is set when flushing"),
                content: std::mem::take(&mut run_content),
                color,
            });

            run_x = cursor_x;
            run_width = 0.0;
        }

        run_style = Some(style);
        run_content.push(ch);
        run_width += ch_width;
        cursor_x += ch_width;
    }

    if let Some(style) = run_style {
        text_items.push(TextItem {
            x: run_x,
            y,
            width: run_width.min(width),
            height,
            font_size,
            style,
            content: run_content,
            color,
        });
    }
}

fn text_style(ch: char) -> TextStyle {
    if requires_advanced_shaping(ch) {
        TextStyle::Advanced
    } else if is_cjk_char(ch) {
        TextStyle::CjkBasic
    } else if is_symbol_font_char(ch) {
        TextStyle::SymbolBasic
    } else {
        TextStyle::MonospaceBasic
    }
}

fn char_cell_width(ch: char) -> u8 {
    if is_wide_cell_char(ch) { 2 } else { 1 }
}

fn is_wide_cell_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115f
            | 0x2329..=0x232a
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe19
            | 0xfe30..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
            | 0x1f300..=0x1faff
            | 0x20000..=0x3fffd
    )
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7af
            | 0xf900..=0xfaff
            | 0xff00..=0xffef
            | 0x20000..=0x3fffd
    )
}

fn is_symbol_font_char(ch: char) -> bool {
    if matches!(ch as u32, 0x2500..=0x257f) {
        return false;
    }

    matches!(
        ch as u32,
        0x2190..=0x27ff | 0x2900..=0x2bff | 0x1f000..=0x1faff
    )
}

fn requires_advanced_shaping(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0590..=0x08ff
            | 0x0900..=0x0dff
            | 0x0e00..=0x0eff
            | 0x0f00..=0x109f
            | 0x1780..=0x18af
            | 0xa800..=0xabff
            | 0xfb50..=0xfdff
            | 0xfe70..=0xfeff
    )
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

    let (left_px, right_px) = snapped_span(x, x + width);
    let (top_px, bottom_px) = snapped_span(y, y + height);
    let left = clip_x(left_px, surface_width);
    let right = clip_x(right_px, surface_width);
    let top = clip_y(top_px, surface_height);
    let bottom = clip_y(bottom_px, surface_height);
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

fn snapped_span(start: f32, end: f32) -> (f32, f32) {
    let start = start.round();
    let mut end = end.round();

    if end <= start {
        end = start + 1.0;
    }

    (start, end)
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
    (font_size * LINE_HEIGHT / TEXT_SIZE).round().max(1.0)
}

fn terminal_cell_width(font_size: f32) -> f32 {
    (font_size * 0.62).round().max(1.0)
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

fn first_available_font_family(font_system: &FontSystem, families: &[&str]) -> Option<String> {
    families
        .iter()
        .find(|family| has_font_family(font_system, family))
        .map(|family| (*family).to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_drawing_uses_monospace_text_style() {
        assert_eq!(text_style('─'), TextStyle::MonospaceBasic);
        assert_eq!(text_style('│'), TextStyle::MonospaceBasic);
        assert_eq!(text_style('╭'), TextStyle::MonospaceBasic);
    }

    #[test]
    fn block_elements_use_symbol_text_style() {
        assert_eq!(text_style('░'), TextStyle::SymbolBasic);
        assert_eq!(text_style('▒'), TextStyle::SymbolBasic);
        assert_eq!(text_style('▓'), TextStyle::SymbolBasic);
    }
}
