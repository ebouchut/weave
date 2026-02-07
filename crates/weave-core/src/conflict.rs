use std::fmt;

/// The type of conflict between two branches' changes to an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both branches modified the same entity and the changes couldn't be merged.
    BothModified,
    /// One branch modified the entity while the other deleted it.
    ModifyDelete { modified_in_ours: bool },
    /// Both branches added an entity with the same ID but different content.
    BothAdded,
}

impl fmt::Display for ConflictKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictKind::BothModified => write!(f, "both modified"),
            ConflictKind::ModifyDelete {
                modified_in_ours: true,
            } => write!(f, "modified in ours, deleted in theirs"),
            ConflictKind::ModifyDelete {
                modified_in_ours: false,
            } => write!(f, "deleted in ours, modified in theirs"),
            ConflictKind::BothAdded => write!(f, "both added"),
        }
    }
}

/// A conflict on a specific entity.
#[derive(Debug, Clone)]
pub struct EntityConflict {
    pub entity_name: String,
    pub entity_type: String,
    pub kind: ConflictKind,
    pub ours_content: Option<String>,
    pub theirs_content: Option<String>,
    pub base_content: Option<String>,
}

impl EntityConflict {
    /// Render this conflict as enhanced conflict markers.
    pub fn to_conflict_markers(&self) -> String {
        let label = format!("{} `{}` ({})", self.entity_type, self.entity_name, self.kind);
        let ours = self.ours_content.as_deref().unwrap_or("");
        let theirs = self.theirs_content.as_deref().unwrap_or("");

        let mut out = String::new();
        out.push_str(&format!("<<<<<<< ours — {}\n", label));
        out.push_str(ours);
        if !ours.is_empty() && !ours.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("=======\n");
        out.push_str(theirs);
        if !theirs.is_empty() && !theirs.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!(">>>>>>> theirs — {}\n", label));
        out
    }
}

/// Statistics about a merge operation.
#[derive(Debug, Clone, Default)]
pub struct MergeStats {
    pub entities_unchanged: usize,
    pub entities_ours_only: usize,
    pub entities_theirs_only: usize,
    pub entities_both_changed_merged: usize,
    pub entities_conflicted: usize,
    pub entities_added_ours: usize,
    pub entities_added_theirs: usize,
    pub entities_deleted: usize,
    pub used_fallback: bool,
    /// Entities that were auto-merged but reference other modified entities.
    pub semantic_warnings: usize,
}

impl MergeStats {
    pub fn has_conflicts(&self) -> bool {
        self.entities_conflicted > 0
    }
}

impl fmt::Display for MergeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unchanged: {}", self.entities_unchanged)?;
        if self.entities_ours_only > 0 {
            write!(f, ", ours-only: {}", self.entities_ours_only)?;
        }
        if self.entities_theirs_only > 0 {
            write!(f, ", theirs-only: {}", self.entities_theirs_only)?;
        }
        if self.entities_both_changed_merged > 0 {
            write!(f, ", auto-merged: {}", self.entities_both_changed_merged)?;
        }
        if self.entities_added_ours > 0 {
            write!(f, ", added-ours: {}", self.entities_added_ours)?;
        }
        if self.entities_added_theirs > 0 {
            write!(f, ", added-theirs: {}", self.entities_added_theirs)?;
        }
        if self.entities_deleted > 0 {
            write!(f, ", deleted: {}", self.entities_deleted)?;
        }
        if self.entities_conflicted > 0 {
            write!(f, ", CONFLICTS: {}", self.entities_conflicted)?;
        }
        if self.semantic_warnings > 0 {
            write!(f, ", semantic-warnings: {}", self.semantic_warnings)?;
        }
        if self.used_fallback {
            write!(f, " (line-level fallback)")?;
        }
        Ok(())
    }
}
