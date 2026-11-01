use std::{
    io::{Write, stdout},
    path::PathBuf,
};

use crate::utils::{get_doc_dir, get_sysroot, parse_rustdoc_filename};
use anyhow::Result;
use walkdir::WalkDir;

pub fn cmd_search(query: &str, crate_name: Option<String>) -> Result<()> {
    let doc_dirs = get_doc_dirs(crate_name)?;

    if doc_dirs.is_empty() {
        anyhow::bail!("No documentation directories found to search.");
    }

    println!("Searching for '{}' in local HTML documentation...", query);
    let mut found = false;

    // Use a hashset to avoid duplicates if different paths point to the same file
    let mut visited = std::collections::HashSet::new();

    for dir in doc_dirs {
        for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("html") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if content.contains(query) {
                        if !visited.insert(path.to_path_buf()) {
                            continue;
                        }
                        found = true;
                        let relative_path = path.strip_prefix(&dir).unwrap_or(path);
                        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                        if let Some((kind, name)) = parse_rustdoc_filename(&file_name) {
                            let mut module_path = relative_path.to_path_buf();
                            module_path.pop(); // remove file name
                            let components: Vec<_> = module_path
                                .components()
                                .map(|c| c.as_os_str().to_string_lossy().to_string())
                                .collect();

                            let crate_name =
                                components.first().map(|s| s.as_str()).unwrap_or("unknown");
                            let file_str = components.join("::");

                            if stdout()
                                .write_all(
                                    format!(
                                        "- [{}] {}::{} ({})\n",
                                        crate_name, file_str, name, kind
                                    )
                                    .as_bytes(),
                                )
                                .is_err()
                            {
                                return Ok(());
                            }
                        } else {
                            if stdout()
                                .write_all(format!("- {}\n", relative_path.display()).as_bytes())
                                .is_err()
                            {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    if !found {
        println!("No matches found.");
    }

    Ok(())
}

fn get_doc_dirs(crate_name: Option<String>) -> Result<Vec<PathBuf>> {
    let mut doc_dirs = Vec::new();

    if let Some(name) = crate_name {
        if let Ok(dir) = get_doc_dir(&name) {
            doc_dirs.push(dir);
        } else {
            anyhow::bail!(
                "Could not find documentation directory for crate '{}'",
                name
            );
        }
    } else {
        // Default: sysroot std docs + current workspace target/doc if exists
        if let Ok(sysroot) = get_sysroot() {
            let mut path = sysroot;
            path.push("share");
            path.push("doc");
            path.push("rust");
            path.push("html");
            if path.exists() {
                doc_dirs.push(path);
            }
        }

        let mut target_doc = std::env::current_dir().unwrap();
        target_doc.push("target");
        target_doc.push("doc");
        if target_doc.exists() {
            doc_dirs.push(target_doc);
        }
    }

    Ok(doc_dirs)
}
