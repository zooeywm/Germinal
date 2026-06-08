use crate::workspace::{PaneId, WorkspaceError, WorkspaceResult};

#[derive(Debug)]
pub struct Workspace {
    tabs: Vec<Tab>,
    active_tab_index: usize,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            tabs: vec![Tab::new()],
            active_tab_index: 0,
        }
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab_index]
    }

    pub fn new_tab(&mut self) {
        self.tabs.push(Tab::new());
        self.active_tab_index = self.tabs.len() - 1;
    }

    pub fn split_focused_pane(&mut self, direction: PaneSplitDirection) -> WorkspaceResult<()> {
        self.active_tab_mut().split_focused_pane(direction)
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab_index]
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Tab {
    pane_tree: PaneTree,
    focused_pane: PaneId,
    next_pane_id: u64,
}

impl Tab {
    pub fn new() -> Self {
        let pane_tree = PaneTree::new();
        let focused_pane = pane_tree.first_pane_id();

        Self {
            pane_tree,
            focused_pane,
            next_pane_id: 2,
        }
    }

    pub fn pane_count(&self) -> usize {
        self.pane_tree.pane_count()
    }

    pub fn focused_pane(&self) -> PaneId {
        self.focused_pane
    }

    fn split_focused_pane(&mut self, direction: PaneSplitDirection) -> WorkspaceResult<()> {
        let new_pane = PaneId::new(self.next_pane_id);
        self.pane_tree
            .split_pane(self.focused_pane, direction, new_pane)?;
        self.focused_pane = new_pane;
        self.next_pane_id += 1;
        Ok(())
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PaneTree {
    root: PaneNode,
}

impl PaneTree {
    pub fn new() -> Self {
        Self {
            root: PaneNode::Pane(Pane::new(PaneId::new(1))),
        }
    }

    pub fn horizontal(children: Vec<Self>) -> Self {
        Self::new_split(PaneSplitDirection::Horizontal, children)
    }

    pub fn vertical(children: Vec<Self>) -> Self {
        Self::new_split(PaneSplitDirection::Vertical, children)
    }

    pub fn pane_count(&self) -> usize {
        self.root.pane_count()
    }

    pub fn first_pane_id(&self) -> PaneId {
        self.root.first_pane_id()
    }

    pub fn split_direction(&self) -> Option<PaneSplitDirection> {
        self.root.split_direction()
    }

    fn split_pane(
        &mut self,
        pane_id: PaneId,
        direction: PaneSplitDirection,
        new_pane_id: PaneId,
    ) -> WorkspaceResult<()> {
        self.root.split_pane(pane_id, direction, new_pane_id)
    }

    fn new_split(direction: PaneSplitDirection, children: Vec<Self>) -> Self {
        Self {
            root: PaneNode::Split {
                direction,
                children: children.into_iter().map(|child| child.root).collect(),
            },
        }
    }
}

impl Default for PaneTree {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum PaneNode {
    Pane(Pane),
    Split {
        direction: PaneSplitDirection,
        children: Vec<PaneNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSplitDirection {
    Horizontal,
    Vertical,
}

impl PaneNode {
    fn pane_count(&self) -> usize {
        match self {
            Self::Pane(_) => 1,
            Self::Split { children, .. } => children.iter().map(Self::pane_count).sum(),
        }
    }

    fn first_pane_id(&self) -> PaneId {
        match self {
            Self::Pane(pane) => pane.id(),
            Self::Split { children, .. } => children[0].first_pane_id(),
        }
    }

    fn split_direction(&self) -> Option<PaneSplitDirection> {
        match self {
            Self::Pane(_) => None,
            Self::Split { direction, .. } => Some(*direction),
        }
    }

    fn split_pane(
        &mut self,
        pane_id: PaneId,
        direction: PaneSplitDirection,
        new_pane_id: PaneId,
    ) -> WorkspaceResult<()> {
        match self {
            Self::Pane(pane) if pane.id() == pane_id => {
                let old_pane = match std::mem::replace(self, Self::Pane(Pane::new(new_pane_id))) {
                    Self::Pane(pane) => pane,
                    Self::Split { .. } => unreachable!(),
                };

                *self = Self::Split {
                    direction,
                    children: vec![Self::Pane(old_pane), Self::Pane(Pane::new(new_pane_id))],
                };

                Ok(())
            }
            Self::Pane(_) => Err(WorkspaceError::PaneNotFound(pane_id)),
            Self::Split { children, .. } => {
                for child in children {
                    if child.split_pane(pane_id, direction, new_pane_id).is_ok() {
                        return Ok(());
                    }
                }

                Err(WorkspaceError::PaneNotFound(pane_id))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pane {
    id: PaneId,
}

impl Pane {
    pub fn new(id: PaneId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> PaneId {
        self.id
    }
}
