use std::path::Path;

use sem_core::parser::registry::ParserRegistry;

use crate::error::Result;
use crate::ops::upsert_entity;
use crate::state::EntityStateDoc;

/// Sync entities from working tree files into CRDT state.
///
/// Extracts entities from each file using sem-core's parser registry,
/// then upserts them into the automerge document.
pub fn sync_from_files(
    state: &mut EntityStateDoc,
    repo_root: &Path,
    file_paths: &[String],
    registry: &ParserRegistry,
) -> Result<usize> {
    let mut count = 0;

    for file_path in file_paths {
        let full_path = repo_root.join(file_path);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue, // File may not exist (deleted)
        };

        let plugin = match registry.get_plugin(file_path) {
            Some(p) => p,
            None => continue, // No parser for this file type
        };

        let entities = plugin.extract_entities(&content, file_path);
        for entity in &entities {
            upsert_entity(
                state,
                &entity.id,
                &entity.name,
                &entity.entity_type,
                file_path,
                &entity.content_hash,
            )?;
            count += 1;
        }
    }

    Ok(count)
}

/// Extract entity IDs from a single file (for lookups).
pub fn extract_entity_ids(
    content: &str,
    file_path: &str,
    registry: &ParserRegistry,
) -> Vec<(String, String, String)> {
    let plugin = match registry.get_plugin(file_path) {
        Some(p) => p,
        None => return Vec::new(),
    };

    plugin
        .extract_entities(content, file_path)
        .into_iter()
        .map(|e| (e.id, e.name, e.entity_type))
        .collect()
}

/// Find entity ID by human-readable name and file path.
pub fn resolve_entity_id(
    content: &str,
    file_path: &str,
    entity_name: &str,
    registry: &ParserRegistry,
) -> Option<String> {
    let entities = extract_entity_ids(content, file_path, registry);
    entities
        .into_iter()
        .find(|(_, name, _)| name == entity_name)
        .map(|(id, _, _)| id)
}
