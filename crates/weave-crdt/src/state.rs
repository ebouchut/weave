use std::path::{Path, PathBuf};

use automerge::{AutoCommit, ObjType, ReadDoc, ROOT, transaction::Transactable};

use crate::error::{Result, WeaveError};

/// Wraps an Automerge document for entity state persistence.
///
/// Document structure:
/// ```text
/// root {
///   entities: Map<entity_id → {
///     name, type, file_path, content_hash,
///     claimed_by, claimed_at,
///     last_modified_by, last_modified_at,
///     version
///   }>,
///   agents: Map<agent_id → {
///     name, status, branch, last_seen,
///     working_on: List<entity_id>
///   }>,
///   operations: List<{agent, entity_id, op, timestamp}>
/// }
/// ```
pub struct EntityStateDoc {
    pub(crate) doc: AutoCommit,
    pub(crate) path: PathBuf,
}

impl EntityStateDoc {
    /// Load from disk or create a new document.
    pub fn open(path: &Path) -> Result<Self> {
        let doc = if path.exists() {
            let data = std::fs::read(path)?;
            AutoCommit::load(&data)?
        } else {
            let mut doc = AutoCommit::new();
            // Initialize top-level structure
            doc.put_object(ROOT, "entities", ObjType::Map)?;
            doc.put_object(ROOT, "agents", ObjType::Map)?;
            doc.put_object(ROOT, "operations", ObjType::List)?;
            doc
        };
        Ok(Self {
            doc,
            path: path.to_path_buf(),
        })
    }

    /// Create a new in-memory document (for testing).
    pub fn new_memory() -> Result<Self> {
        let mut doc = AutoCommit::new();
        doc.put_object(ROOT, "entities", ObjType::Map)?;
        doc.put_object(ROOT, "agents", ObjType::Map)?;
        doc.put_object(ROOT, "operations", ObjType::List)?;
        Ok(Self {
            doc,
            path: PathBuf::new(),
        })
    }

    /// Save the document to disk.
    pub fn save(&mut self) -> Result<()> {
        if self.path.as_os_str().is_empty() {
            return Ok(()); // In-memory mode, no-op
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = self.doc.save();
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    /// Get the ExId of the "entities" map.
    pub(crate) fn entities_id(&self) -> Result<automerge::ObjId> {
        match self.doc.get(ROOT, "entities")? {
            Some((_, id)) => Ok(id),
            None => Err(WeaveError::Automerge(
                automerge::AutomergeError::InvalidObjId("entities map missing".into()),
            )),
        }
    }

    /// Get the ExId of the "agents" map.
    pub(crate) fn agents_id(&self) -> Result<automerge::ObjId> {
        match self.doc.get(ROOT, "agents")? {
            Some((_, id)) => Ok(id),
            None => Err(WeaveError::Automerge(
                automerge::AutomergeError::InvalidObjId("agents map missing".into()),
            )),
        }
    }

    /// Get the ExId of the "operations" list.
    pub(crate) fn operations_id(&self) -> Result<automerge::ObjId> {
        match self.doc.get(ROOT, "operations")? {
            Some((_, id)) => Ok(id),
            None => Err(WeaveError::Automerge(
                automerge::AutomergeError::InvalidObjId("operations list missing".into()),
            )),
        }
    }
}

/// Get current time in milliseconds since epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
