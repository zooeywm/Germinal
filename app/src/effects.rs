use std::{
    cell::RefCell,
    collections::VecDeque,
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};

use germinal_application::runtime::RuntimeEffect;
use germinal_domain::gshell::GShellId;
use germinal_infra::{
    pty::{UnixPty, UnixPtyReader, UnixPtyWriter},
    window::WinitWindowEventProxy,
};
use germinal_ports::{pty::PtyResult, window::WindowEventProxy};

use crate::container::GerminalApp;

pub struct RuntimeEffectExecutor {
    write_queue: Rc<RefCell<PtyWriteQueue>>,
    read_state: Rc<RefCell<PtyReadState>>,
}

struct PtyWriteQueue {
    commands: VecDeque<PtyWriteCommand>,
    waker: Option<Waker>,
}

struct PtyReadState {
    window_proxy: Option<WinitWindowEventProxy>,
}

struct PtyWriteWorker {
    queue: Rc<RefCell<PtyWriteQueue>>,
    pty_writer: UnixPtyWriter,
}

struct PtyReadLoop {
    state: Rc<RefCell<PtyReadState>>,
    pty_reader: UnixPtyReader,
    active_id: GShellId,
}

struct PtyWriteCommand {
    id: GShellId,
    bytes: Vec<u8>,
}

impl RuntimeEffectExecutor {
    pub fn new(initial_id: GShellId) -> PtyResult<Self> {
        let (pty_reader, pty_writer) = UnixPty::spawn_split(initial_id)?;

        let write_queue = Rc::new(RefCell::new(PtyWriteQueue {
            commands: VecDeque::new(),
            waker: None,
        }));
        let read_state = Rc::new(RefCell::new(PtyReadState { window_proxy: None }));

        compio::runtime::spawn(
            PtyWriteWorker {
                queue: write_queue.clone(),
                pty_writer,
            }
            .run(),
        )
        .detach();

        compio::runtime::spawn(
            PtyReadLoop {
                state: read_state.clone(),
                pty_reader,
                active_id: initial_id,
            }
            .run(),
        )
        .detach();

        Ok(Self {
            write_queue,
            read_state,
        })
    }

    pub fn set_window_event_proxy(&mut self, proxy: WinitWindowEventProxy) {
        self.read_state.borrow_mut().window_proxy = Some(proxy);
    }

    pub fn apply(&mut self, app: &mut GerminalApp, effects: Vec<RuntimeEffect>) {
        for effect in effects {
            match effect {
                RuntimeEffect::WritePty { id, bytes } => {
                    if !app.as_ref().contains(id) {
                        eprintln!("failed to write PTY input: session not found");
                        continue;
                    }

                    self.enqueue_write(PtyWriteCommand { id, bytes });
                }
            }
        }
    }

    fn enqueue_write(&self, command: PtyWriteCommand) {
        let waker = {
            let mut queue = self.write_queue.borrow_mut();
            queue.commands.push_back(command);
            queue.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl PtyWriteWorker {
    async fn run(mut self) {
        loop {
            let command = self.next_command().await;

            if let Err(err) = self.pty_writer.write(command.id, &command.bytes).await {
                eprintln!("failed to write PTY input: {err:?}");
            }
        }
    }

    async fn next_command(&self) -> PtyWriteCommand {
        poll_fn(|cx| {
            let mut queue = self.queue.borrow_mut();

            if let Some(command) = queue.commands.pop_front() {
                Poll::Ready(command)
            } else {
                queue.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }
}

impl PtyReadLoop {
    async fn run(mut self) {
        loop {
            match self.pty_reader.read(self.active_id).await {
                Ok(bytes) if bytes.is_empty() => {}
                Ok(_bytes) => {
                    if let Some(proxy) = self.state.borrow().window_proxy.clone() {
                        proxy.notify_pty_output();
                    }
                }
                Err(err) => {
                    eprintln!("failed to read PTY output: {err:?}");
                    break;
                }
            }
        }
    }
}
