//! The collaboration seam: every mutation of a document is an `Op` — a set of
//! position-based text edits plus intent metadata. Ops are serializable and
//! map 1:1 onto CRDT text splices (automerge `splice_text`) for phase 2 sync.

use crate::model::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    /// Byte range in the current text to replace.
    pub start: usize,
    pub end: usize,
    pub insert: String,
}

impl TextEdit {
    pub fn new(start: usize, end: usize, insert: impl Into<String>) -> Self {
        TextEdit { start, end, insert: insert.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Typing,
    MoveNode(NodeId),
    MoveGroup(NodeId),
    Paste,
    FileLoad,
}

pub type ActorId = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Op {
    /// Applied atomically, back-to-front (descending start offset), so ranges
    /// refer to the text as it was before the op.
    pub edits: Vec<TextEdit>,
    pub intent: Intent,
    pub actor: ActorId,
    pub seq: u64,
}
