use germinal_application::gshell::{RunningGShell, read_pty};
use germinal_infra::pty::UnixPty;

/// Reads PTY output until the target bytes appear.
///
/// This is still smoke-test code, not the final runtime pump.
pub async fn pump_pty_output_until(
    pty: &mut UnixPty,
    shell: &mut RunningGShell,
    target: &[u8],
) -> Vec<u8> {
    let mut collected = Vec::new();

    while !contains_bytes(&collected, target) {
        let output = read_pty(pty, shell)
            .await
            .expect("failed to read PTY output");

        collected.extend_from_slice(&output);
    }

    collected
}

fn contains_bytes(bytes: &[u8], target: &[u8]) -> bool {
    bytes.windows(target.len()).any(|window| window == target)
}
