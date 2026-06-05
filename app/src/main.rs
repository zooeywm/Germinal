use std::thread;
use std::time::Duration;

use germinal_application::{
    gshell::{close_pty_gshell, read_pty_output, start_pty_gshell, write_pty_input},
    rendering::render_frame,
};
use germinal_domain::{
    gshell::GShellId,
    rendering::{Color, RenderCommand, RenderFrame},
};
use germinal_infra::{pty::UnixPty, renderer::FakeRenderer};

fn main() {
    let mut pty = UnixPty::new();
    let shell = start_pty_gshell(&mut pty, GShellId::new(1)).expect("failed to start PTY GShell");

    write_pty_input(&mut pty, &shell, b"echo hello germinal\n").expect("failed to write PTY input");

    let mut last_byte = b'\n';

    for _ in 0..10 {
        let output = read_pty_output(&mut pty, &shell).expect("failed to read PTY output");

        if !output.is_empty() {
            if let Some(byte) = output.last() {
                last_byte = *byte;
            }

            print!("{}", String::from_utf8_lossy(&output));
        }

        thread::sleep(Duration::from_millis(100));
    }

    if last_byte != b'\n' {
        println!();
    }

    close_pty_gshell(&mut pty, shell).expect("failed to close PTY GShell");

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
