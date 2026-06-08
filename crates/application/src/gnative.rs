use germinal_domain::{
    gshell::{Color as SurfaceColor, GNativeAppName, GNativeSurface, GRequest, SurfaceCommand},
    rendering::{Color as RenderColor, RenderCommand, RenderFrame},
};

pub struct DemoGNativeApp;

impl DemoGNativeApp {
    pub fn new() -> Self {
        Self
    }

    pub fn surface(&mut self) -> GNativeSurface {
        let mut surface = GNativeSurface::new();

        surface.push(SurfaceCommand::Clear(SurfaceColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }));

        surface.push(SurfaceCommand::Text {
            x: 16.0,
            y: 24.0,
            content: "Hello Demo GNativeApp".to_string(),
            color: SurfaceColor {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        });

        surface
    }
}

impl Default for DemoGNativeApp {
    fn default() -> Self {
        Self::new()
    }
}

fn create_demo_surface(request: GRequest) -> Option<GNativeSurface> {
    match request {
        GRequest::EnterGNative { app } if is_demo_app(&app) => {
            let mut app = DemoGNativeApp::new();
            Some(app.surface())
        }
        _ => None,
    }
}

fn is_demo_app(app: &GNativeAppName) -> bool {
    app.as_str() == "demo"
}

fn compose_surface_to_frame(surface: &GNativeSurface) -> RenderFrame {
    let mut frame = RenderFrame::new();

    for command in surface.commands() {
        match command {
            SurfaceCommand::Clear(color) => {
                frame.push(RenderCommand::Clear(RenderColor {
                    r: color.r,
                    g: color.g,
                    b: color.b,
                    a: color.a,
                }));
            }
            SurfaceCommand::FillRect {
                x,
                y,
                width,
                height,
                color,
            } => {
                frame.push(RenderCommand::FillRect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                    color: RenderColor {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: color.a,
                    },
                });
            }
            SurfaceCommand::Text {
                x,
                y,
                content,
                color,
            } => {
                let width = content.chars().count() as f32 * 9.0;
                frame.push(RenderCommand::Text {
                    x: *x,
                    y: *y,
                    width,
                    height: 18.0,
                    font_size: 15.0,
                    content: content.clone(),
                    color: RenderColor {
                        r: color.r,
                        g: color.g,
                        b: color.b,
                        a: color.a,
                    },
                });
            }
        }
    }

    frame
}

pub fn render_gnative_request(request: GRequest) -> Option<RenderFrame> {
    let surface = create_demo_surface(request)?;
    Some(compose_surface_to_frame(&surface))
}
