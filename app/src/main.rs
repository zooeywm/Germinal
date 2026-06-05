mod container;

use germinal_application::{gshell::GShellService, rendering::render_frame};
use germinal_domain::rendering::{Color, RenderCommand, RenderFrame};
use germinal_infra::renderer::FakeRenderer;

use crate::container::GerminalApp;

#[compio::main]
async fn main() {
    let mut app = GerminalApp::new();

    let gshell_service = GShellService::inj_ref_mut(&mut app);

    let shell_id = gshell_service.spawn().expect("failed to start PTY GShell");

    gshell_service
        .write_pty(shell_id, b"echo hello germinal\n")
        .await
        .expect("failed to write PTY input");

    let target = b"\r\nhello germinal\r\n";
    let mut output = Vec::new();

    while !output.windows(target.len()).any(|window| window == target) {
        let bytes = gshell_service
            .read_active_pty()
            .await
            .expect("failed to read PTY output");

        output.extend_from_slice(&bytes);
    }

    if !output.is_empty() {
        print!("{}", String::from_utf8_lossy(&output));

        if output.last() != Some(&b'\n') {
            println!();
        }
    }

    gshell_service
        .close(shell_id)
        .expect("failed to close PTY GShell");

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
