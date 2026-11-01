use crate::utils::{self, RustPath, fetch_docs_rs, find_html_file, get_doc_dir};
use anyhow::Result;
use htmd::{
    Element,
    element_handler::{HandlerResult, Handlers},
};
use scraper::{Html, Selector};
use std::path::PathBuf;

pub fn cmd_doc(path: &str, signatures_only: bool) -> Result<()> {
    let parsed_path = RustPath::parse(path).map_err(|e| anyhow::anyhow!(e))?;

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

    let markdown = html_to_markdown(
        &html_content,
        &base_url_or_path,
        is_local,
        member.as_deref(),
        signatures_only,
    )?;
    utils::write(&format!("{}\n", markdown))?;
    Ok(())
}

fn html_to_markdown(
    html_content: &str,
    base_url_or_path: &str,
    is_local: bool,
    member: Option<&str>,
    signatures_only: bool,
) -> Result<String> {
    let document = Html::parse_document(html_content);

    let base_url = if is_local {
        let path = PathBuf::from(base_url_or_path);
        let absolute = std::fs::canonicalize(&path).unwrap_or(path);
        reqwest::Url::from_file_path(absolute).map_err(|_| anyhow::anyhow!("Invalid file path"))?
    } else {
        reqwest::Url::parse(base_url_or_path)?
    };

    let mut selected_html = None;
    if let Some(m) = member {
        let escaped_m = m.replace(".", "\\.");
        let sel = Selector::parse(&format!(r#"[id$=".{}"]"#, escaped_m)).unwrap();
        if let Some(el) = document.select(&sel).next() {
            let mut container = el;
            if let Some(parent) = container.parent() {
                if let Some(parent_el) = parent.value().as_element() {
                    if parent_el.name() == "summary" {
                        if let Some(grandparent) = parent.parent() {
                            if grandparent.value().is_element() {
                                container = scraper::ElementRef::wrap(grandparent).unwrap();
                            }
                        }
                    }
                }
            }
            selected_html = Some(container.html());
        }
    }

    let final_html = if signatures_only && selected_html.is_none() {
        let mut combined = String::new();
        let decl_sel = Selector::parse(r#".item-decl"#).unwrap();
        if let Some(decl) = document.select(&decl_sel).next() {
            combined.push_str(&decl.html());
        }

        let sig_sel =
            Selector::parse(r#".impl-items .code-header, .variants .code-header"#).unwrap();
        for sig in document.select(&sig_sel) {
            combined.push_str(&format!("<div>{}</div>", sig.html()));
        }

        if combined.is_empty() {
            anyhow::bail!("Content not found in HTML");
        }
        combined
    } else if let Some(h) = selected_html {
        h
    } else {
        let main_sel = Selector::parse(r#"main#main-content, .rustdoc"#).unwrap();
        if let Some(main) = document.select(&main_sel).next() {
            main.inner_html()
        } else {
            anyhow::bail!("Content not found in HTML");
        }
    };

    let extracted_html = rewrite_urls(&final_html, &base_url);

    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "nav"])
        .add_handler(vec!["a"], |a: &dyn Handlers, b: Element| {
            let mut content = String::new();
            for child in b.node.children.borrow().iter() {
                if let Some(result) = a.handle(child) {
                    content.push_str(&result.content);
                }
            }
            if let Some(title) = b.attrs.iter().find_map(|x| {
                if x.name.local.as_bytes() == b"title" {
                    Some(x.value.clone())
                } else {
                    None
                }
            }) {
                content = format!("[{}]({})", content, title);
            }
            Some(HandlerResult {
                content,
                markdown_translated: true,
            })
        })
        .build();

    let markdown = converter
        .convert(&extracted_html)
        .map_err(|e| anyhow::anyhow!("htmd error: {:?}", e))?;

    Ok(markdown)
}

fn rewrite_urls(html_content: &str, base_url: &reqwest::Url) -> String {
    let href_re = regex::Regex::new(r#"href="([^"]+)""#).unwrap();
    let src_re = regex::Regex::new(r#"src="([^"]+)""#).unwrap();

    let extracted_html = href_re
        .replace_all(html_content, |c: &regex::Captures| {
            let link = &c[1];
            if link.starts_with("javascript:")
                || link.starts_with("mailto:")
                || link.starts_with("data:")
            {
                return format!(r#"href="{}""#, link);
            }
            if let Ok(resolved) = base_url.join(link) {
                format!(r#"href="{}""#, resolved)
            } else {
                format!(r#"href="{}""#, link)
            }
        })
        .to_string();

    let extracted_html = src_re
        .replace_all(&extracted_html, |c: &regex::Captures| {
            let link = &c[1];
            if link.starts_with("data:") {
                return format!(r#"src="{}""#, link);
            }
            if let Ok(resolved) = base_url.join(link) {
                format!(r#"src="{}""#, resolved)
            } else {
                format!(r#"src="{}""#, link)
            }
        })
        .to_string();

    extracted_html
}

#[test]
fn test_doc() -> Result<()> {
    cmd_doc("std::vec::Vec", false)?;
    Ok(())
}
