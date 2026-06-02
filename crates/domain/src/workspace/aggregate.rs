use crate::workspace::{PaneSplitDirection, Workspace, WorkspaceResult};

#[derive(Debug)]
pub struct Window {
    workspace: Workspace,
}

impl Window {
    pub fn new() -> Self {
        Self {
            workspace: Workspace::new(),
        }
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn new_tab(&mut self) {
        self.workspace.new_tab();
    }

    pub fn split_focused_pane(&mut self, direction: PaneSplitDirection) -> WorkspaceResult<()> {
        self.workspace.split_focused_pane(direction)
    }
}

impl Default for Window {
    fn default() -> Self {
        Self::new()
    }
}
