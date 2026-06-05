use std::collections::HashMap;
use std::env;
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

use germinal_ports::pty::{PtyHandle, PtyPort, PtySize};
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
    fn spawn(&mut self) -> PtyHandle {
        let pty = openpty(None, None).expect("failed to open PTY");

        let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());

        let user_fd = pty
            .user
            .as_fd()
            .try_clone_to_owned()
            .expect("failed to clone PTY user fd");

        let pty_controller = pty.controller;

        let flags = fcntl_getfl(&pty_controller).expect("failed to get PTY flags");
        fcntl_setfl(&pty_controller, flags | OFlags::NONBLOCK)
            .expect("failed to set PTY nonblocking");

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

        let child = command.spawn().expect("failed to spawn shell");

        let id = self.next_id;
        self.next_id += 1;

        self.sessions.insert(
            id,
            UnixPtySession {
                pty_controller,
                child,
            },
        );

        PtyHandle::new(id)
    }

    fn write(&mut self, handle: &PtyHandle, bytes: &[u8]) {
        let id = handle.id();
        let session = self.sessions.get(&id).expect("unknown PTY handle");

        write(&session.pty_controller, bytes).expect("failed to write to PTY");
    }

    fn read(&mut self, handle: &PtyHandle) -> Vec<u8> {
        let id = handle.id();
        let session = self.sessions.get(&id).expect("unknown PTY handle");

        let mut buffer = vec![0; 4096];

        match read(&session.pty_controller, &mut buffer) {
            Ok(read_len) => {
                buffer.truncate(read_len);
                buffer
            }
            Err(Errno::AGAIN) => Vec::new(),
            Err(err) => panic!("failed to read from PTY: {err}"),
        }
    }

    fn resize(&mut self, handle: &PtyHandle, size: PtySize) {
        let id = handle.id();
        let session = self.sessions.get(&id).expect("unknown PTY handle");

        let winsize = Winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        tcsetwinsize(&session.pty_controller, winsize).expect("failed to resize PTY");
    }

    fn close(&mut self, handle: PtyHandle) {
        let id = handle.id();

        let Some(mut session) = self.sessions.remove(&id) else {
            return;
        };

        let _ = session.child.kill();
        let _ = session.child.wait();
    }
}
