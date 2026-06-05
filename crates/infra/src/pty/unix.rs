use std::collections::HashMap;
use std::env;
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

use germinal_ports::pty::{PtyError, PtyHandle, PtyPort, PtyResult, PtySize};
use rustix::fd::OwnedFd;
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::io::{Errno, read, write};
use rustix::termios::{Winsize, tcsetwinsize};
use rustix_openpty::{login_tty, openpty};

/// Unix PTY implementation.
///
/// This owns real PTY resources on Unix-like systems.
/// Each resource is addressed by a PtyHandle.
pub struct UnixPty {
    next_id: u64,
    sessions: HashMap<u64, UnixPtySession>,
}

impl UnixPty {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            sessions: HashMap::new(),
        }
    }
}

impl Default for UnixPty {
    fn default() -> Self {
        Self::new()
    }
}

/// One Unix PTY session.
///
/// controller is the terminal-emulator side kept by Germinal.
/// The user side is handed to the child process during spawn.
struct UnixPtySession {
    pty_controller: OwnedFd,
    child: Child,
}

impl PtyPort for UnixPty {
    fn spawn(&mut self) -> PtyResult<PtyHandle> {
        let pty = openpty(None, None).map_err(|_| PtyError::SpawnFailed)?;

        let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());

        let user_fd = pty
            .user
            .as_fd()
            .try_clone_to_owned()
            .map_err(|_| PtyError::SpawnFailed)?;

        let pty_controller = pty.controller;

        let flags = fcntl_getfl(&pty_controller).map_err(|_| PtyError::SpawnFailed)?;
        fcntl_setfl(&pty_controller, flags | OFlags::NONBLOCK)
            .map_err(|_| PtyError::SpawnFailed)?;

        // Drop the parent-owned user side. The child gets its own cloned fd.
        drop(pty.user);

        let mut user_fd = Some(user_fd);

        let pre_exec = move || {
            // pre_exec requires FnMut, but login_tty consumes OwnedFd.
            // Store it in Option and take it once in the child process.
            let user_fd = user_fd
                .take()
                .ok_or_else(|| std::io::Error::other("PTY user fd already used"))?;

            login_tty(user_fd).map_err(|err| std::io::Error::from_raw_os_error(err.raw_os_error()))
        };

        let mut command = Command::new(shell);

        // SAFETY: pre_exec registers a closure that runs in the child process
        // after fork and before exec. The closure only installs the PTY user fd
        // as the controlling terminal.
        unsafe {
            command.pre_exec(pre_exec);
        }

        let child = command.spawn().map_err(|_| PtyError::SpawnFailed)?;

        let id = self.next_id;
        self.next_id += 1;

        self.sessions.insert(
            id,
            UnixPtySession {
                pty_controller,
                child,
            },
        );

        Ok(PtyHandle::new(id))
    }

    fn write(&mut self, handle: &PtyHandle, bytes: &[u8]) -> PtyResult<()> {
        let id = handle.id();
        let session = self.sessions.get(&id).ok_or(PtyError::UnknownHandle)?;

        write(&session.pty_controller, bytes).map_err(|_| PtyError::IoFailed)?;

        Ok(())
    }

    fn read(&mut self, handle: &PtyHandle) -> PtyResult<Vec<u8>> {
        let id = handle.id();
        let session = self.sessions.get(&id).ok_or(PtyError::UnknownHandle)?;

        let mut buffer = vec![0; 4096];

        match read(&session.pty_controller, &mut buffer) {
            Ok(read_len) => {
                buffer.truncate(read_len);
                Ok(buffer)
            }
            Err(Errno::AGAIN) => Ok(Vec::new()),
            Err(_) => Err(PtyError::IoFailed),
        }
    }

    fn resize(&mut self, handle: &PtyHandle, size: PtySize) -> PtyResult<()> {
        let id = handle.id();
        let session = self.sessions.get(&id).ok_or(PtyError::UnknownHandle)?;

        let winsize = Winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        tcsetwinsize(&session.pty_controller, winsize).map_err(|_| PtyError::IoFailed)?;

        Ok(())
    }

    fn close(&mut self, handle: PtyHandle) -> PtyResult<()> {
        let id = handle.id();

        let Some(mut session) = self.sessions.remove(&id) else {
            return Err(PtyError::UnknownHandle);
        };

        let _ = session.child.kill();
        let _ = session.child.wait();

        Ok(())
    }
}
