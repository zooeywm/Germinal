use std::{
    cell::RefCell,
    collections::VecDeque,
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};

use germinal_application::runtime::RuntimeEffect;
use germinal_domain::gshell::GShellId;
use germinal_infra::pty::UnixPty;
use germinal_ports::pty::{PtyPort, PtyResult};

use crate::container::GerminalApp;

pub struct RuntimeEffectExecutor {
    shared: Rc<RefCell<RuntimeEffectExecutorShared>>,
}

struct RuntimeEffectExecutorShared {
    commands: VecDeque<RuntimeIoCommand>,
    waker: Option<Waker>,
}

struct RuntimeEffectIoRunner {
    shared: Rc<RefCell<RuntimeEffectExecutorShared>>,
    pty_backend: UnixPty,
}

enum RuntimeIoCommand {
    WritePty { id: GShellId, bytes: Vec<u8> },
}

impl RuntimeEffectExecutor {
    pub fn new(initial_id: GShellId) -> PtyResult<Self> {
        let mut pty_backend = UnixPty::new();
        pty_backend.spawn(initial_id)?;

        let shared = Rc::new(RefCell::new(RuntimeEffectExecutorShared {
            commands: VecDeque::new(),
            waker: None,
        }));

        let runner = RuntimeEffectIoRunner {
            shared: shared.clone(),
            pty_backend,
        };

        compio::runtime::spawn(runner.run()).detach();

        Ok(Self { shared })
    }

    pub fn apply(&mut self, app: &mut GerminalApp, effects: Vec<RuntimeEffect>) {
        for effect in effects {
            match effect {
                RuntimeEffect::WritePty { id, bytes } => {
                    if !app.as_ref().contains(id) {
                        eprintln!("failed to write PTY input: session not found");
                        continue;
                    }

                    self.enqueue_command(RuntimeIoCommand::WritePty { id, bytes });
                }
            }
        }
    }

    fn enqueue_command(&self, command: RuntimeIoCommand) {
        let waker = {
            let mut shared = self.shared.borrow_mut();
            shared.commands.push_back(command);
            shared.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl RuntimeEffectIoRunner {
    async fn run(mut self) {
        loop {
            match self.next_command().await {
                RuntimeIoCommand::WritePty { id, bytes } => {
                    if let Err(err) = self.pty_backend.write(id, &bytes).await {
                        eprintln!("failed to write PTY input: {err:?}");
                    }
                }
            }
        }
    }

    async fn next_command(&self) -> RuntimeIoCommand {
        poll_fn(|cx| {
            let mut shared = self.shared.borrow_mut();

            if let Some(command) = shared.commands.pop_front() {
                Poll::Ready(command)
            } else {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await
    }
}
