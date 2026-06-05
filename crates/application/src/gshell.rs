use germinal_domain::gshell::{GShell, GShellId};
use germinal_ports::pty::{PtyHandle, PtyPort, PtyResult, PtySize};

/// Runtime binding maintained by the application layer.
///
/// GShell is domain state.
///
/// PtyHandle is a reference to the real PTY/ConPTY external resource.
/// The binding lives in application so domain does not depend on ports or infra.
pub struct RunningGShell {
    pub shell: GShell,
    pub pty: PtyHandle,
}

/// Switches the running GShell to GNativeMode.
///
/// This only changes domain state.
/// Starting the real GNativeApp process or protocol connection is handled separately.
pub fn enter_gnative_mode(running: &mut RunningGShell) {
    running.shell.enter_gnative();
}

/// Switches the running GShell back to PtyMode.
///
/// This only changes domain state.
/// Real GNativeApp cleanup is handled separately by application/infra.
pub fn exit_gnative_mode(running: &mut RunningGShell) {
    running.shell.exit_gnative();
}

/// Starts a GShell in PtyMode through a PTY port.
pub fn start_pty_gshell(pty_port: &mut impl PtyPort, id: GShellId) -> PtyResult<RunningGShell> {
    let shell = GShell::new(id);
    let pty = pty_port.spawn()?;

    Ok(RunningGShell { shell, pty })
}

/// Writes input bytes to the PTY bound to this running GShell.
pub async fn write_pty(
    pty_port: &mut impl PtyPort,
    running: &RunningGShell,
    bytes: &[u8],
) -> PtyResult<()> {
    pty_port.write(&running.pty, bytes).await
}

/// Reads output bytes from the PTY bound to this running GShell.
pub async fn read_pty(
    pty_port: &mut impl PtyPort,
    running: &mut RunningGShell,
) -> PtyResult<Vec<u8>> {
    let bytes = pty_port.read(&running.pty).await?;

    running.shell.apply_pty_output_bytes(&bytes);

    Ok(bytes)
}

/// Resizes the PTY bound to this running GShell.
pub fn resize_pty(
    pty_port: &mut impl PtyPort,
    running: &RunningGShell,
    size: PtySize,
) -> PtyResult<()> {
    pty_port.resize(&running.pty, size)
}

/// Closes the PTY bound to this running GShell.
pub fn close_pty_gshell(pty_port: &mut impl PtyPort, running: RunningGShell) -> PtyResult<()> {
    pty_port.close(running.pty)
}
