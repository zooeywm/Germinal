use crate::workspace::PaneId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    PaneNotFound(PaneId),
    CannotCloseLastPane,
}

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;
