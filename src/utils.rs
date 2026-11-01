use anyhow::{Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct RustPath {
    pub crate_name: String,
    pub modules: Vec<String>,
}

impl RustPath {
    pub fn parse(path: &str) -> std::result::Result<Self, &'static str> {
        let parts: Vec<&str> = path.split("::").filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("Path cannot be empty");
        }

        let crate_name = parts[0].to_string();
        let modules = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok(RustPath {
            crate_name,
            modules,
        })
    }
}

pub fn is_std_crate(crate_name: &str) -> bool {
    matches!(crate_name, "std" | "core" | "alloc" | "test" | "proc_macro")
}

pub fn get_sysroot() -> Result<PathBuf> {
    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()
        .context("Failed to run rustc")?;

    let sysroot = String::from_utf8(output.stdout)?.trim().to_string();

    Ok(PathBuf::from(sysroot))
}

pub fn get_doc_dir(crate_name: &str) -> Result<PathBuf> {
    if is_std_crate(crate_name) {
        let mut path = get_sysroot()?;
        path.push("share");
        path.push("doc");
        path.push("rust");
        path.push("html");
        Ok(path)
    } else {
        // Run cargo doc in the current workspace if it's stale
        let mut target_dir = std::env::current_dir()?;
        target_dir.push("target");

        let hash_file = target_dir.join(".rdocdown_hash");
        let doc_dir = target_dir.join("doc");

        let lock_file = std::env::current_dir()?.join("Cargo.lock");
        let toml_file = std::env::current_dir()?.join("Cargo.toml");

        let contents_to_hash = if lock_file.exists() {
            std::fs::read_to_string(&lock_file).unwrap_or_default()
        } else if toml_file.exists() {
            std::fs::read_to_string(&toml_file).unwrap_or_default()
        } else {
            String::new()
        };

        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        contents_to_hash.hash(&mut hasher);
        let current_hash = hasher.finish().to_string();

        let is_stale = if hash_file.exists() && doc_dir.exists() {
            let stored_hash = std::fs::read_to_string(&hash_file).unwrap_or_default();
            stored_hash.trim() != current_hash
        } else {
            true
        };

        if is_stale {
            println!("Building documentation index...");
            let status = Command::new("cargo")
                .arg("doc")
                .arg("--document-private-items")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .context("Failed to run cargo doc")?;

            if !status.success() {
                anyhow::bail!("cargo doc failed");
            }

            if !target_dir.exists() {
                std::fs::create_dir_all(&target_dir)?;
            }
            std::fs::write(&hash_file, current_hash)?;
        }

        Ok(doc_dir)
    }
}

pub fn find_html_file(base_dir: &Path, path: &RustPath) -> Result<(PathBuf, Option<String>)> {
    if let Ok(p) = find_html_file_exact(base_dir, path) {
        return Ok((p, None));
    }

    if !path.modules.is_empty() {
        let mut parent_path = path.clone();
        let member = parent_path.modules.pop().unwrap();
        if let Ok(p) = find_html_file_exact(base_dir, &parent_path) {
            return Ok((p, Some(member)));
        }
    }

    anyhow::bail!("Could not find HTML file for path {:?}", path);
}

fn find_html_file_exact(base_dir: &Path, path: &RustPath) -> Result<PathBuf> {
    let mut current_dir = base_dir.to_path_buf();
    current_dir.push(&path.crate_name);

    if path.modules.is_empty() {
        current_dir.push("index.html");
        if current_dir.exists() {
            return Ok(current_dir);
        }
        anyhow::bail!("Could not find index.html for crate {}", path.crate_name);
    }

    let (item_name, mod_path) = path.modules.split_last().unwrap();

    for mod_name in mod_path {
        current_dir.push(mod_name);
    }

    let prefixes = [
        "struct", "enum", "trait", "type", "fn", "macro", "constant", "union", "keyword",
    ];

    // check if it's a module
    let mut mod_dir = current_dir.clone();
    mod_dir.push(item_name);
    mod_dir.push("index.html");
    if mod_dir.exists() {
        return Ok(mod_dir);
    }

    // check prefixes
    for prefix in prefixes {
        let file_name = format!("{}.{}.html", prefix, item_name);
        let mut file_path = current_dir.clone();
        file_path.push(&file_name);
        if file_path.exists() {
            return Ok(file_path);
        }
    }

    anyhow::bail!("Could not find HTML file for path {:?}", path);
}

// Returns (HTML Content, URL, Option<MemberName>)
pub fn fetch_docs_rs(path: &RustPath) -> Result<(String, String, Option<String>)> {
    if let Ok((content, url)) = fetch_docs_rs_exact(path) {
        return Ok((content, url, None));
    }

    if !path.modules.is_empty() {
        let mut parent_path = path.clone();
        let member = parent_path.modules.pop().unwrap();
        if let Ok((content, url)) = fetch_docs_rs_exact(&parent_path) {
            return Ok((content, url, Some(member)));
        }
    }

    anyhow::bail!(
        "Could not find documentation on docs.rs for path {:?}",
        path
    );
}

fn fetch_docs_rs_exact(path: &RustPath) -> Result<(String, String)> {
    let base_url = format!(
        "https://docs.rs/{}/latest/{}",
        path.crate_name, path.crate_name
    );

    if path.modules.is_empty() {
        let url = format!("{}/index.html", base_url);
        let resp = reqwest::blocking::get(&url)?;
        if resp.status().is_success() {
            return Ok((resp.text()?, url));
        }
        anyhow::bail!(
            "Could not find index.html on docs.rs for crate {}",
            path.crate_name
        );
    }

    let (item_name, mod_path) = path.modules.split_last().unwrap();
    let mod_url_path = mod_path.join("/");

    let dir_url = if mod_url_path.is_empty() {
        base_url.clone()
    } else {
        format!("{}/{}", base_url, mod_url_path)
    };

    // check if it's a module
    let mod_url = format!("{}/{}/index.html", dir_url, item_name);
    let resp = reqwest::blocking::get(&mod_url)?;
    if resp.status().is_success() {
        return Ok((resp.text()?, mod_url));
    }

    let prefixes = [
        "struct", "enum", "trait", "type", "fn", "macro", "constant", "union", "keyword",
    ];

    for prefix in prefixes {
        let url = format!("{}/{}.{}.html", dir_url, prefix, item_name);
        let resp = reqwest::blocking::get(&url)?;
        if resp.status().is_success() {
            return Ok((resp.text()?, url));
        }
    }

    anyhow::bail!(
        "Could not find documentation on docs.rs for path {:?}",
        path
    );
}

pub fn parse_rustdoc_filename(filename: &str) -> Option<(String, String)> {
    if !filename.ends_with(".html") {
        return None;
    }
    let parts: Vec<&str> = filename.split('.').collect();
    if parts.len() >= 3 {
        // e.g. struct.Config.html
        let kind = parts[0];
        let name = parts[1..parts.len() - 1].join(".");
        Some((kind.to_string(), name))
    } else {
        None
    }
}

pub fn write(s: &str) -> Result<()> {
    if let Err(e) = std::io::stdout().write_all(s.as_bytes()) {
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            return Err(e.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rustpath_parse() {
        let p = RustPath::parse("std::vec::Vec").unwrap();
        assert_eq!(p.crate_name, "std");
        assert_eq!(p.modules, vec!["vec", "Vec"]);

        let p2 = RustPath::parse("anyhow").unwrap();
        assert_eq!(p2.crate_name, "anyhow");
        assert!(p2.modules.is_empty());

        assert!(RustPath::parse("").is_err());
        // Handles leading/trailing colons
        let p3 = RustPath::parse("::std::vec::").unwrap();
        assert_eq!(p3.crate_name, "std");
        assert_eq!(p3.modules, vec!["vec"]);
    }

    #[test]
    fn test_parse_rustdoc_filename() {
        assert_eq!(
            parse_rustdoc_filename("struct.Vec.html"),
            Some(("struct".to_string(), "Vec".to_string()))
        );
        assert_eq!(
            parse_rustdoc_filename("fn.is_std_crate.html"),
            Some(("fn".to_string(), "is_std_crate".to_string()))
        );
        assert_eq!(parse_rustdoc_filename("index.html"), None);
        assert_eq!(parse_rustdoc_filename("struct.Vec.txt"), None);
    }
}
