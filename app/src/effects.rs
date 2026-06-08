use germinal_application::{gshell::GShellService, runtime::RuntimeEffect};

use crate::container::GerminalApp;

pub struct RuntimeEffectExecutor;

impl RuntimeEffectExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn apply(&mut self, app: &mut GerminalApp, effects: Vec<RuntimeEffect>) {
        for effect in effects {
            match effect {
                RuntimeEffect::WritePty { id, bytes } => {
                    let result = {
                        let gshell_service = GShellService::inj_ref_mut(app);
                        gshell_service.write_pty(id, &bytes).await
                    };

                    if let Err(err) = result {
                        eprintln!("failed to write PTY input: {err:?}");
                    }
                }
            }
        }
    }
}

impl Default for RuntimeEffectExecutor {
    fn default() -> Self {
        Self::new()
    }
}
