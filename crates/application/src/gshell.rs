use germinal_domain::gshell::{GShell, GShellId};
use germinal_ports::pty::{PtyHandle, PtyPort, PtySize};

/// Runtime binding maintained by the application layer.
///
/// GShell is domain state.
///
/// PtyHandle is a reference to the real PTY/ConPTY external resource.
/// The binding lives in application so domain does not depend on ports or infra.s
pub struct RunningGShell {
    pub shell: GShell,
    pub pty: PtyHandle,
}

/// Starts a GShell in PtyMode and binds it to a real PTY resource.
///
/// The GShell identity is provided by the caller.
/// The actual PTY/ConPTY resource is created through PtyPort.
pub fn start_pty_gshell(pty_port: &mut impl PtyPort, id: GShellId) -> RunningGShell {
    let shell = GShell::new(id);
    let pty = pty_port.spawn();

    RunningGShell { shell, pty }
}

/// Writes input bytes to the PTY bound to this running GShell.
pub fn write_pty_input(pty_port: &mut impl PtyPort, running: &RunningGShell, bytes: &[u8]) {
    pty_port.write(&running.pty, bytes);
}

/// Reads output bytes from the PTY bound to this running GShell.
pub fn read_pty_output(pty_port: &mut impl PtyPort, running: &RunningGShell) -> Vec<u8> {
    pty_port.read(&running.pty)
}

/// Resizes the PTY bound to this running GShell.
pub fn resize_pty(pty_port: &mut impl PtyPort, running: &RunningGShell, size: PtySize) {
    pty_port.resize(&running.pty, size);
}

/// Closes the PTY bound to this running GShell.
pub fn close_pty_gshell(pty_port: &mut impl PtyPort, running: RunningGShell) {
    pty_port.close(running.pty);
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
