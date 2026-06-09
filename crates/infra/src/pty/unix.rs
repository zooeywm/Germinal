use std::collections::HashMap;
use std::env;
use std::os::fd::AsFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

use germinal_ports::pty::{GShellId, PtyError, PtyPort, PtyResult, PtySize};
use rustix::fd::OwnedFd;
use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::termios::{Winsize, tcsetwinsize};
use rustix_openpty::{login_tty, openpty};

const DEFAULT_PTY_COLS: u16 = 80;
const DEFAULT_PTY_ROWS: u16 = 24;
const PTY_READ_BUFFER_SIZE: usize = 64 * 1024;

use compio::buf::BufResult;
use compio::io::{AsyncRead, AsyncWrite};
use compio::runtime::fd::AsyncFd;

/// Unix PTY implementation.
///
/// This owns real PTY resources on Unix-like systems.
/// Each resource is addressed by a GShellId.
pub struct UnixPty {
    reader: UnixPtyReader,
    writer: UnixPtyWriter,
}

pub struct UnixPtyReader {
    controllers: HashMap<GShellId, AsyncFd<OwnedFd>>,
}

pub struct UnixPtyWriter {
    sessions: HashMap<GShellId, UnixPtyWriteSession>,
}

impl UnixPty {
    pub fn new() -> Self {
        Self {
            reader: UnixPtyReader::new(),
            writer: UnixPtyWriter::new(),
        }
    }

    pub fn spawn_split(initial_id: GShellId) -> PtyResult<(UnixPtyReader, UnixPtyWriter)> {
        let mut pty = Self::new();
        pty.spawn(initial_id)?;
        Ok(pty.split())
    }

    pub fn split(self) -> (UnixPtyReader, UnixPtyWriter) {
        (self.reader, self.writer)
    }
}

impl Default for UnixPty {
    fn default() -> Self {
        Self::new()
    }
}

struct UnixPtyWriteSession {
    pty_controller: AsyncFd<OwnedFd>,
    child: Child,
}

impl UnixPtyReader {
    fn new() -> Self {
        Self {
            controllers: HashMap::new(),
        }
    }

    pub async fn read(&mut self, id: GShellId) -> PtyResult<Vec<u8>> {
        let pty_controller = self
            .controllers
            .get_mut(&id)
            .ok_or(PtyError::SessionNotFound)?;

        read_pty_controller(pty_controller).await
    }

    fn insert(&mut self, id: GShellId, pty_controller: AsyncFd<OwnedFd>) -> PtyResult<()> {
        if self.controllers.contains_key(&id) {
            return Err(PtyError::SessionAlreadyExists);
        }

        self.controllers.insert(id, pty_controller);

        Ok(())
    }

    fn close(&mut self, id: GShellId) -> PtyResult<()> {
        self.controllers
            .remove(&id)
            .map(|_| ())
            .ok_or(PtyError::SessionNotFound)
    }
}

impl UnixPtyWriter {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub async fn write(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<()> {
        let session = self
            .sessions
            .get_mut(&id)
            .ok_or(PtyError::SessionNotFound)?;

        write_pty_controller(&mut session.pty_controller, bytes).await
    }

    fn insert(&mut self, id: GShellId, session: UnixPtyWriteSession) -> PtyResult<()> {
        if self.sessions.contains_key(&id) {
            return Err(PtyError::SessionAlreadyExists);
        }

        self.sessions.insert(id, session);

        Ok(())
    }

    pub fn resize(&mut self, id: GShellId, size: PtySize) -> PtyResult<()> {
        let session = self.sessions.get(&id).ok_or(PtyError::SessionNotFound)?;

        let winsize = Winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        tcsetwinsize(&session.pty_controller, winsize).map_err(|_| PtyError::IoFailed)?;

        Ok(())
    }

    fn close(&mut self, id: GShellId) -> PtyResult<()> {
        let Some(mut session) = self.sessions.remove(&id) else {
            return Err(PtyError::SessionNotFound);
        };

        let _ = session.child.kill();
        let _ = session.child.try_wait();

        Ok(())
    }
}

impl Drop for UnixPtyWriter {
    fn drop(&mut self) {
        for session in self.sessions.values_mut() {
            let _ = session.child.kill();
            let _ = session.child.try_wait();
        }
    }
}

impl PtyPort for UnixPty {
    fn spawn(&mut self, id: GShellId) -> PtyResult<()> {
        let (read_controller, write_session) = spawn_session_pair()?;

        self.reader.insert(id, read_controller)?;
        self.writer.insert(id, write_session)?;

        Ok(())
    }

    async fn write(&mut self, id: GShellId, bytes: &[u8]) -> PtyResult<()> {
        self.writer.write(id, bytes).await
    }

    async fn read(&mut self, id: GShellId) -> PtyResult<Vec<u8>> {
        self.reader.read(id).await
    }

    fn resize(&mut self, id: GShellId, size: PtySize) -> PtyResult<()> {
        self.writer.resize(id, size)
    }

    fn close(&mut self, id: GShellId) -> PtyResult<()> {
        self.reader.close(id)?;
        self.writer.close(id)
    }
}

fn spawn_session_pair() -> PtyResult<(AsyncFd<OwnedFd>, UnixPtyWriteSession)> {
    let winsize = Winsize {
        ws_row: DEFAULT_PTY_ROWS,
        ws_col: DEFAULT_PTY_COLS,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let pty = openpty(None, Some(&winsize)).map_err(|_| PtyError::SpawnFailed)?;

    let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());

    let user_fd = pty
        .user
        .as_fd()
        .try_clone_to_owned()
        .map_err(|_| PtyError::SpawnFailed)?;

    let pty_controller = pty.controller;

    let flags = fcntl_getfl(&pty_controller).map_err(|_| PtyError::SpawnFailed)?;
    fcntl_setfl(&pty_controller, flags | OFlags::NONBLOCK).map_err(|_| PtyError::SpawnFailed)?;

    let read_controller = pty_controller
        .as_fd()
        .try_clone_to_owned()
        .map_err(|_| PtyError::SpawnFailed)?;

    let read_controller = AsyncFd::new(read_controller).map_err(|_| PtyError::SpawnFailed)?;
    let write_controller = AsyncFd::new(pty_controller).map_err(|_| PtyError::SpawnFailed)?;

    drop(pty.user);

    let mut user_fd = Some(user_fd);

    let pre_exec = move || {
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

    Ok((
        read_controller,
        UnixPtyWriteSession {
            pty_controller: write_controller,
            child,
        },
    ))
}

async fn write_pty_controller(
    pty_controller: &mut AsyncFd<OwnedFd>,
    bytes: &[u8],
) -> PtyResult<()> {
    let mut written = 0;

    while written < bytes.len() {
        let buffer = bytes[written..].to_vec();
        let BufResult(result, _buffer) = pty_controller.write(buffer).await;

        let n = result.map_err(|_| PtyError::IoFailed)?;

        if n == 0 {
            return Err(PtyError::IoFailed);
        }

        written += n;
    }

    Ok(())
}

async fn read_pty_controller(pty_controller: &mut AsyncFd<OwnedFd>) -> PtyResult<Vec<u8>> {
    let buffer = vec![0; PTY_READ_BUFFER_SIZE];
    let BufResult(result, mut buffer) = pty_controller.read(buffer).await;

    let read_len = result.map_err(|_| PtyError::IoFailed)?;
    buffer.truncate(read_len);

    Ok(buffer)
}
