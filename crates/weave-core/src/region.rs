use sem_core::model::entity::SemanticEntity;

/// A region of a file — either an entity or the interstitial content between entities.
#[derive(Debug, Clone)]
pub enum FileRegion {
    Entity(EntityRegion),
    Interstitial(InterstitialRegion),
}

impl FileRegion {
    pub fn content(&self) -> &str {
        match self {
            FileRegion::Entity(e) => &e.content,
            FileRegion::Interstitial(i) => &i.content,
        }
    }

    pub fn key(&self) -> &str {
        match self {
            FileRegion::Entity(e) => &e.entity_id,
            FileRegion::Interstitial(i) => &i.position_key,
        }
    }

    pub fn is_entity(&self) -> bool {
        matches!(self, FileRegion::Entity(_))
    }
}

#[derive(Debug, Clone)]
pub struct EntityRegion {
    pub entity_id: String,
    pub entity_name: String,
    pub entity_type: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct InterstitialRegion {
    /// A key like "before:entity_id" or "after:entity_id" or "file_header" / "file_footer"
    pub position_key: String,
    pub content: String,
}

/// Extract ordered regions from file content using the given entities.
///
/// Entities must be from the same file. The function splits the file into
/// alternating interstitial and entity regions based on line ranges.
pub fn extract_regions(content: &str, entities: &[SemanticEntity]) -> Vec<FileRegion> {
    if entities.is_empty() {
        // Entire file is one interstitial region
        return vec![FileRegion::Interstitial(InterstitialRegion {
            position_key: "file_only".to_string(),
            content: content.to_string(),
        })];
    }

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Sort entities by start_line (they should already be sorted, but be safe)
    let mut sorted_entities: Vec<&SemanticEntity> = entities.iter().collect();
    sorted_entities.sort_by_key(|e| e.start_line);

    let mut regions: Vec<FileRegion> = Vec::new();
    let mut current_line: usize = 0; // 0-indexed into lines array

    for (i, entity) in sorted_entities.iter().enumerate() {
        // Entity start_line and end_line are 1-based from sem-core
        let entity_start = entity.start_line.saturating_sub(1); // convert to 0-based
        let entity_end = entity.end_line; // end_line is inclusive, so this is exclusive in 0-based

        // Interstitial before this entity
        if current_line < entity_start {
            let interstitial_content = join_lines(&lines[current_line..entity_start]);
            let position_key = if i == 0 {
                "file_header".to_string()
            } else {
                format!("between:{}:{}", sorted_entities[i - 1].id, entity.id)
            };
            regions.push(FileRegion::Interstitial(InterstitialRegion {
                position_key,
                content: interstitial_content,
            }));
        }

        // Entity region — use the entity's own content (which sem-core extracts accurately)
        // but also compute from lines for consistency
        let entity_end_clamped = entity_end.min(total_lines);
        let entity_content = if entity_start < entity_end_clamped {
            join_lines(&lines[entity_start..entity_end_clamped])
        } else {
            entity.content.clone()
        };

        regions.push(FileRegion::Entity(EntityRegion {
            entity_id: entity.id.clone(),
            entity_name: entity.name.clone(),
            entity_type: entity.entity_type.clone(),
            content: entity_content,
            start_line: entity.start_line,
            end_line: entity.end_line,
        }));

        current_line = entity_end_clamped;
    }

    // Interstitial after last entity (file footer)
    if current_line < total_lines {
        let footer_content = join_lines(&lines[current_line..total_lines]);
        regions.push(FileRegion::Interstitial(InterstitialRegion {
            position_key: "file_footer".to_string(),
            content: footer_content,
        }));
    }

    // Handle trailing newline — if original content ends with newline and our last region doesn't
    if content.ends_with('\n') {
        if let Some(last) = regions.last() {
            if !last.content().ends_with('\n') {
                match regions.last_mut() {
                    Some(FileRegion::Entity(e)) => e.content.push('\n'),
                    Some(FileRegion::Interstitial(i)) => i.content.push('\n'),
                    None => {}
                }
            }
        }
    }

    regions
}

fn join_lines(lines: &[&str]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut result = lines.join("\n");
    result.push('\n');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use sem_core::parser::plugins::create_default_registry;

    #[test]
    fn test_extract_regions_typescript() {
        let content = r#"import { foo } from 'bar';

export function hello() {
    return "hello";
}

export function world() {
    return "world";
}
"#;

        let registry = create_default_registry();
        let plugin = registry.get_plugin("test.ts").unwrap();
        let entities = plugin.extract_entities(content, "test.ts");

        assert!(!entities.is_empty(), "Should extract entities from TypeScript");

        let regions = extract_regions(content, &entities);

        // Should have interstitial + entity regions
        assert!(regions.len() >= 2, "Should have multiple regions, got {}", regions.len());

        // Verify entities are present
        let entity_regions: Vec<_> = regions
            .iter()
            .filter_map(|r| match r {
                FileRegion::Entity(e) => Some(e),
                _ => None,
            })
            .collect();

        let entity_names: Vec<&str> = entity_regions.iter().map(|e| e.entity_name.as_str()).collect();
        assert!(entity_names.contains(&"hello"), "Should find hello function, got {:?}", entity_names);
        assert!(entity_names.contains(&"world"), "Should find world function, got {:?}", entity_names);
    }

    #[test]
    fn test_extract_regions_no_entities() {
        let content = "just some text\nno code here\n";
        let regions = extract_regions(content, &[]);
        assert_eq!(regions.len(), 1);
        assert!(!regions[0].is_entity());
        assert_eq!(regions[0].content(), content);
    }
}
