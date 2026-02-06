use std::fs;
use std::process;

use weave_core::entity_merge;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    // Git calls: weave-driver %O %A %B %L %P
    // %O = ancestor (base), %A = current (ours), %B = other (theirs)
    // %L = conflict marker size, %P = file path
    if args.len() < 4 {
        eprintln!("Usage: weave-driver <base> <ours> <theirs> [marker-size] [file-path]");
        eprintln!("  Typically invoked by git as a merge driver.");
        process::exit(2);
    }

    let base_path = &args[1];
    let ours_path = &args[2];
    let theirs_path = &args[3];
    // args[4] is marker size (unused, we use our own markers)
    let file_path = if args.len() > 5 {
        args[5].clone()
    } else if args.len() > 4 {
        // Sometimes %P comes as 4th positional
        args[4].clone()
    } else {
        ours_path.clone()
    };

    // Read input files
    let base = match fs::read_to_string(base_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("weave: failed to read base file '{}': {}", base_path, e);
            process::exit(2);
        }
    };
    let ours = match fs::read_to_string(ours_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("weave: failed to read ours file '{}': {}", ours_path, e);
            process::exit(2);
        }
    };
    let theirs = match fs::read_to_string(theirs_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("weave: failed to read theirs file '{}': {}", theirs_path, e);
            process::exit(2);
        }
    };

    // Detect binary content (null bytes in first 8KB)
    if is_binary(&base) || is_binary(&ours) || is_binary(&theirs) {
        eprintln!("weave: binary file detected, skipping entity merge for '{}'", file_path);
        process::exit(2);
    }

    // Run entity merge
    let result = entity_merge(&base, &ours, &theirs, &file_path);

    // Write result to ours path (git convention: merge driver writes to %A)
    if let Err(e) = fs::write(ours_path, &result.content) {
        eprintln!("weave: failed to write result to '{}': {}", ours_path, e);
        process::exit(2);
    }

    // Print stats to stderr
    eprintln!("weave [{}]: {}", file_path, result.stats);

    // Optionally record merge in CRDT state
    #[cfg(feature = "crdt")]
    record_merge_in_crdt(&file_path, &result.content);

    if result.is_clean() {
        process::exit(0);
    } else {
        eprintln!(
            "weave: {} conflict(s) in '{}'",
            result.conflicts.len(),
            file_path
        );
        for conflict in &result.conflicts {
            eprintln!(
                "  - {} `{}`: {}",
                conflict.entity_type, conflict.entity_name, conflict.kind
            );
        }
        process::exit(1);
    }
}

fn is_binary(content: &str) -> bool {
    content.as_bytes().iter().take(8192).any(|&b| b == 0)
}

/// Record merge results in CRDT state if `.weave/state.automerge` exists.
/// Fails silently — this is purely advisory and must never break the merge.
#[cfg(feature = "crdt")]
fn record_merge_in_crdt(file_path: &str, _merged_content: &str) {
    let _ = (|| -> Result<(), Box<dyn std::error::Error>> {
        let repo_root = weave_core::git::find_repo_root()?;
        let state_path = repo_root.join(".weave").join("state.automerge");
        if !state_path.exists() {
            return Ok(());
        }
        let mut state = weave_crdt::EntityStateDoc::open(&state_path)?;
        let registry = sem_core::parser::plugins::create_default_registry();
        weave_crdt::sync_from_files(&mut state, &repo_root, &[file_path.to_string()], &registry)?;
        state.save()?;
        Ok(())
    })();
}
