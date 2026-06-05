mod pty_pump;

use std::time::Duration;

use compio::time::sleep;
use germinal_application::{
    gshell::{close_pty_gshell, start_pty_gshell, write_pty},
    rendering::render_frame,
};
use germinal_domain::{
    gshell::GShellId,
    rendering::{Color, RenderCommand, RenderFrame},
};
use germinal_infra::{pty::UnixPty, renderer::FakeRenderer};

#[compio::main]
async fn main() {
    let mut pty = UnixPty::new();
    let mut shell =
        start_pty_gshell(&mut pty, GShellId::new(1)).expect("failed to start PTY GShell");

    sleep(Duration::from_millis(300)).await;

    write_pty(&mut pty, &shell, b"echo hello germinal\n")
        .await
        .expect("failed to write PTY input");

    let output =
        pty_pump::pump_pty_output_until(&mut pty, &mut shell, b"\r\nhello germinal\r\n").await;

    if !output.is_empty() {
        print!("{}", String::from_utf8_lossy(&output));

        if output.last() != Some(&b'\n') {
            println!();
        }
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
