use std::process::Command;

use colored::Colorize;
use sem_core::parser::plugins::create_default_registry;
use weave_core::entity_merge_with_registry;

pub fn run(
    branch: &str,
    file_path: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Find merge base
    let head = "HEAD";
    let merge_base_output = Command::new("git")
        .args(["merge-base", head, branch])
        .output()?;
    if !merge_base_output.status.success() {
        return Err(format!(
            "Failed to find merge base between HEAD and '{}'. Are both branches valid?",
            branch
        )
        .into());
    }
    let merge_base = String::from_utf8_lossy(&merge_base_output.stdout)
        .trim()
        .to_string();

    // Get list of files changed in either branch
    let files = if let Some(fp) = file_path {
        vec![fp.to_string()]
    } else {
        get_changed_files(&merge_base, head, branch)?
    };

    if files.is_empty() {
        println!("{} No files with changes in both branches.", "✓".green().bold());
        return Ok(());
    }

    let registry = create_default_registry();
    let mut total_conflicts = 0;
    let mut total_auto_resolved = 0;

    for file in &files {
        let base_content = git_show(&merge_base, file).unwrap_or_default();
        let ours_content = git_show(head, file).unwrap_or_default();
        let theirs_content = git_show(branch, file).unwrap_or_default();

        // Skip if no three-way difference
        if ours_content == theirs_content || base_content == ours_content || base_content == theirs_content {
            continue;
        }

        let result = entity_merge_with_registry(
            &base_content,
            &ours_content,
            &theirs_content,
            file,
            &registry,
        );

        let status = if result.is_clean() {
            total_auto_resolved += 1;
            format!("{}", "auto-resolved".green())
        } else {
            total_conflicts += result.conflicts.len();
            format!("{} conflict(s)", result.conflicts.len().to_string().red().bold())
        };

        println!("  {} — {}", file, status);
        println!("    {}", result.stats);

        for conflict in &result.conflicts {
            println!(
                "    {} {} `{}`: {}",
                "✗".red(),
                conflict.entity_type,
                conflict.entity_name,
                conflict.kind
            );
        }
    }

    println!();
    if total_conflicts == 0 {
        println!(
            "{} Merge would be clean ({} file(s) auto-resolved by weave)",
            "✓".green().bold(),
            total_auto_resolved
        );
    } else {
        println!(
            "{} Merge would have {} entity-level conflict(s) ({} file(s) auto-resolved)",
            "✗".red().bold(),
            total_conflicts,
            total_auto_resolved
        );
    }

    Ok(())
}

fn get_changed_files(
    merge_base: &str,
    head: &str,
    branch: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Files changed in ours
    let ours_output = Command::new("git")
        .args(["diff", "--name-only", merge_base, head])
        .output()?;
    let ours_files: std::collections::HashSet<String> =
        String::from_utf8_lossy(&ours_output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

    // Files changed in theirs
    let theirs_output = Command::new("git")
        .args(["diff", "--name-only", merge_base, branch])
        .output()?;
    let theirs_files: std::collections::HashSet<String> =
        String::from_utf8_lossy(&theirs_output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

    // Intersection — files changed in both branches
    let mut both: Vec<String> = ours_files
        .intersection(&theirs_files)
        .cloned()
        .collect();
    both.sort();
    Ok(both)
}

fn git_show(rev: &str, file: &str) -> Result<String, Box<dyn std::error::Error>> {
    let spec = format!("{}:{}", rev, file);
    let output = Command::new("git")
        .args(["show", &spec])
        .output()?;
    if !output.status.success() {
        return Err(format!("git show {} failed", spec).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
