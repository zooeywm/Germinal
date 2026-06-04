use germinal_application::{
    gshell::{read_pty_output, start_pty_gshell, write_pty_input},
    rendering::render_frame,
};
use germinal_domain::{
    gshell::GShellId,
    rendering::{Color, RenderCommand, RenderFrame},
};
use germinal_infra::{pty::FakePty, renderer::FakeRenderer};

fn main() {
    let mut pty = FakePty::new();
    let shell = start_pty_gshell(&mut pty, GShellId::new(1));

    write_pty_input(&mut pty, &shell, b"hello germinal");

    let output = read_pty_output(&mut pty, &shell);
    println!("{}", String::from_utf8_lossy(&output));

    let mut frame = RenderFrame::new();
    frame.push(RenderCommand::Clear(Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    }));
    frame.push(RenderCommand::Text {
        x: 16.0,
        y: 24.0,
        content: "Hello GNativeMode".to_string(),
        color: Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    });

    let mut renderer = FakeRenderer;
    render_frame(&mut renderer, &frame);
}
