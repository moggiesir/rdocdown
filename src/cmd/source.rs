use crate::utils::{self, RustPath, fetch_docs_rs, find_html_file, get_doc_dir};
use anyhow::Result;
use scraper::{Html, Selector};
use std::path::PathBuf;

pub fn cmd_source(path_str: &str) -> Result<()> {
    let parsed_path = RustPath::parse(path_str).map_err(|e| anyhow::anyhow!(e))?;

    let (html_content, base_url_or_path, is_local, member) =
        match get_doc_dir(&parsed_path.crate_name) {
            Ok(doc_dir) => match find_html_file(&doc_dir, &parsed_path) {
                Ok((html_file, member)) => (
                    std::fs::read_to_string(&html_file)?,
                    html_file.to_string_lossy().to_string(),
                    true,
                    member,
                ),
                Err(_) => {
                    let (content, url, member) = fetch_docs_rs(&parsed_path)?;
                    (content, url, false, member)
                }
            },
            Err(_) => {
                let (content, url, member) = fetch_docs_rs(&parsed_path)?;
                (content, url, false, member)
            }
        };

    let document = Html::parse_document(&html_content);
    let mut found_src_href = None;

    if let Some(m) = member {
        let escaped_m = m.replace(".", "\\.");
        let sel = Selector::parse(&format!(
            r#"[id$=".{}"] a.src, [id$=".{}"] a.srclink, [id$=".{}"] a[class*="src"]"#,
            escaped_m, escaped_m, escaped_m
        ))
        .unwrap();
        if let Some(a) = document.select(&sel).next() {
            if let Some(href) = a.value().attr("href") {
                found_src_href = Some(href.to_string());
            }
        }
    }

    if found_src_href.is_none() {
        let selector = Selector::parse(r#"a.src, a.srclink, a[class*="src"]"#).unwrap();
        for a in document.select(&selector) {
            if let Some(href) = a.value().attr("href") {
                found_src_href = Some(href.to_string());
                break;
            }
        }
    }

    let mut fragment = None;
    if let Some(mut href) = found_src_href {
        if let Some(idx) = href.find('#') {
            fragment = Some(href[idx + 1..].to_string());
            href.truncate(idx);
        }
        let source_html = if is_local {
            let mut local_path = PathBuf::from(&base_url_or_path);
            local_path.pop(); // remove current file name
            local_path.push(href);
            let resolved = std::fs::canonicalize(local_path)?;
            std::fs::read_to_string(resolved)?
        } else {
            let base_url = reqwest::Url::parse(&base_url_or_path)?;
            let resolved_url = base_url.join(&href)?;
            reqwest::blocking::get(resolved_url)?.text()?
        };

        let src_doc = Html::parse_document(&source_html);
        // Usually source is in `<div class="example-wrap"><pre class="rust"><code>`
        // Or in older rustdoc versions, `<pre class="rust">`
        let code_sel = Selector::parse(
            r#"pre.rust, pre.src-line-numbers ~ pre.rust, div.example-wrap pre.rust"#,
        )
        .unwrap();
        if let Some(code_elem) = src_doc.select(&code_sel).next() {
            let mut code_text: String = code_elem
                .text()
                // Replace some common rustdoc HTML entities if scraper doesn't (it usually does)
                .collect::<Vec<_>>()
                .join("");

            if let Some(frag) = fragment {
                code_text = extract_lines(&code_text, &frag);
            }

            utils::write(&format!("```rust\n{}\n```\n", code_text))?;
            return Ok(());
        } else {
            // Fallback for docs.rs which might use diff classes
            let fallback_sel = Selector::parse(r#"main"#).unwrap();
            if let Some(main_elem) = src_doc.select(&fallback_sel).next() {
                let code_text: String = main_elem.text().collect::<Vec<_>>().join("");
                utils::write(&format!("```rust\n{}\n```\n", code_text))?;
                return Ok(());
            }
            anyhow::bail!("Could not find code block in source HTML");
        }
    }

    anyhow::bail!(
        "Could not find source link in documentation for {}",
        path_str
    );
}

fn extract_lines(code: &str, fragment: &str) -> String {
    let parts: Vec<&str> = fragment.split('-').collect();
    let start: usize = match parts.first().and_then(|s| s.parse().ok()) {
        Some(s) => s,
        None => return code.to_string(), // Failed to parse, not a line number range
    };
    let end: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(start);

    let lines: Vec<&str> = code.lines().collect();
    let start_idx = start.saturating_sub(1);
    let end_idx = end.min(lines.len());

    if start_idx >= lines.len() {
        return code.to_string(); // In case of weird parse error or mismatch, return all
    }

    lines[start_idx..end_idx].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_lines() {
        let code = "line 1\nline 2\nline 3\nline 4\nline 5";

        // Single line
        assert_eq!(extract_lines(code, "3"), "line 3");

        // Range
        assert_eq!(extract_lines(code, "2-4"), "line 2\nline 3\nline 4");

        // Out of bounds end
        assert_eq!(extract_lines(code, "4-10"), "line 4\nline 5");

        // Out of bounds start
        assert_eq!(extract_lines(code, "10-20"), code); // Should return full code on mismatch

        // Invalid parse
        assert_eq!(
            extract_lines(code, "abc"),
            "line 1\nline 2\nline 3\nline 4\nline 5"
        );
    }
}
