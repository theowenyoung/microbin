use ammonia::Builder;
use comrak::{markdown_to_html, Options};
use std::collections::HashSet;

/// Detected content type for auto-detection
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Markdown,
    Html,
    PlainText,
}

/// Detect content type from text content
pub fn detect_content_type(content: &str) -> ContentType {
    // Check for HTML first (more specific patterns)
    if is_html_content(content) {
        return ContentType::Html;
    }

    // Check for Markdown patterns
    if is_markdown_content(content) {
        return ContentType::Markdown;
    }

    ContentType::PlainText
}

fn is_html_content(content: &str) -> bool {
    let trimmed = content.trim();

    // Check for DOCTYPE or html tag at start
    if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<!doctype") {
        return true;
    }
    if trimmed.starts_with("<html") || trimmed.starts_with("<HTML") {
        return true;
    }

    // Check for multiple HTML block elements (not just inline)
    let html_block_tags = [
        "<div", "<section", "<article", "<header", "<footer", "<nav", "<main", "<aside", "<table",
        "<form", "<body", "<head",
    ];
    let lower_content = content.to_lowercase();
    let block_count = html_block_tags
        .iter()
        .filter(|tag| lower_content.contains(*tag))
        .count();

    block_count >= 2
}

fn is_markdown_content(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let mut md_indicators = 0;

    for line in &lines {
        let trimmed = line.trim();

        // Headers (# followed by space)
        if trimmed.starts_with('#') {
            let after_hashes: String = trimmed.chars().skip_while(|c| *c == '#').collect();
            if after_hashes.starts_with(' ') {
                md_indicators += 2;
            }
        }
        // Code blocks
        if trimmed.starts_with("```") {
            md_indicators += 2;
        }
        // Unordered lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            md_indicators += 1;
        }
        // Ordered lists (digit followed by . and space)
        if let Some(first_char) = trimmed.chars().next() {
            if first_char.is_ascii_digit() && trimmed.contains(". ") {
                let parts: Vec<&str> = trimmed.splitn(2, ". ").collect();
                if parts.len() == 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
                    md_indicators += 1;
                }
            }
        }
        // Links/images [text](url) or ![alt](url)
        if trimmed.contains('[') && trimmed.contains("](") {
            md_indicators += 1;
        }
        // Bold/italic
        if trimmed.contains("**") || trimmed.contains("__") {
            md_indicators += 1;
        }
        // Blockquotes
        if trimmed.starts_with("> ") {
            md_indicators += 1;
        }
        // Tables (line starts and ends with |)
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            md_indicators += 2;
        }
    }

    // Require minimum threshold relative to content size
    let threshold = (lines.len() / 10).max(2);
    md_indicators >= threshold
}

/// Extract frontmatter from content if present
/// Frontmatter must be at the very beginning: ---\n...content...\n---\n
fn extract_frontmatter(content: &str) -> (Option<String>, &str) {
    // Must start with exactly "---" followed by a newline
    let after_open = if content.starts_with("---\n") {
        &content[4..]
    } else if content.starts_with("---\r\n") {
        &content[5..]
    } else {
        return (None, content);
    };

    // Find the FIRST closing --- (must be at start of line)
    // Handle both \n--- and \r\n---
    let close_pos = after_open
        .find("\n---")
        .or_else(|| after_open.find("\r\n---"));

    if let Some(pos) = close_pos {
        // Determine if it was \n--- or \r\n---
        let marker_len = if after_open[pos..].starts_with("\r\n---") { 5 } else { 4 };
        let rest = &after_open[pos + marker_len..];

        // Closing --- must be followed by newline or EOF
        if rest.is_empty() || rest.starts_with('\n') || rest.starts_with('\r') {
            let frontmatter = after_open[..pos].trim();
            if !frontmatter.is_empty() {
                let remaining = rest.trim_start_matches(['\r', '\n']);
                return (Some(frontmatter.to_string()), remaining);
            }
        }
    }

    (None, content)
}

/// Render markdown to safe HTML
pub fn render_markdown(content: &str) -> String {
    let mut options = Options::default();

    // Extension options (GFM and more)
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    options.extension.tagfilter = true;
    options.extension.header_ids = Some("".to_owned());
    options.extension.multiline_block_quotes = true;
    options.extension.underline = true;
    options.extension.spoiler = true;
    options.extension.greentext = true;

    // Render options
    options.render.unsafe_ = false; // Don't allow raw HTML in markdown
    options.render.github_pre_lang = true; // Use GitHub-style language class on pre tags

    // Extract and render frontmatter separately
    let (frontmatter, remaining_content) = extract_frontmatter(content);

    let mut html = String::new();

    // Render frontmatter as YAML code block if present
    if let Some(ref fm) = frontmatter {
        html.push_str("<pre><code class=\"language-yaml\">");
        html.push_str(&html_escape::encode_text(fm));
        html.push_str("</code></pre>\n");
    }

    // Render the rest of the markdown
    html.push_str(&markdown_to_html(remaining_content, &options));

    // Sanitize output
    sanitize_html(&html)
}

/// Sanitize HTML for safe display
pub fn sanitize_html(content: &str) -> String {
    let mut allowed_classes = HashSet::new();
    allowed_classes.insert("language-");

    Builder::default()
        .add_tags(&[
            "table",
            "thead",
            "tbody",
            "tr",
            "th",
            "td",
            "pre",
            "code",
            "blockquote",
            "hr",
            "del",
            "sup",
            "sub",
            "input",
            // For footnotes
            "section",
            "ol",
            "li",
        ])
        .add_tag_attributes("a", &["href", "title", "id", "class"]) // id for footnote refs
        .add_tag_attributes("img", &["src", "alt", "title"])
        .add_tag_attributes("code", &["class"])
        .add_tag_attributes("input", &["type", "checked", "disabled"]) // For task lists
        .add_tag_attributes("li", &["id"]) // For footnote definitions
        .add_tag_attributes("section", &["class"]) // For footnotes section
        .add_tag_attributes("sup", &["class", "id"]) // For footnote refs
        .url_schemes(HashSet::from(["http", "https", "mailto"]))
        .link_rel(Some("noopener noreferrer"))
        .clean(content)
        .to_string()
}

/// Prepare HTML content for iframe display (escape for srcdoc attribute)
pub fn prepare_html_for_iframe(content: &str) -> String {
    html_escape::encode_double_quoted_attribute(content).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_markdown_headers() {
        let md = "# Header\n\nSome **bold** text\n\n- List item";
        assert_eq!(detect_content_type(md), ContentType::Markdown);
    }

    #[test]
    fn test_detect_markdown_code_fence() {
        let md = "Some text\n\n```rust\nfn main() {}\n```\n\nMore text";
        assert_eq!(detect_content_type(md), ContentType::Markdown);
    }

    #[test]
    fn test_detect_html_doctype() {
        let html = "<!DOCTYPE html>\n<html><body>Hello</body></html>";
        assert_eq!(detect_content_type(html), ContentType::Html);
    }

    #[test]
    fn test_detect_html_multiple_blocks() {
        let html = "<div>Content</div>\n<section>More</section>\n<article>Article</article>";
        assert_eq!(detect_content_type(html), ContentType::Html);
    }

    #[test]
    fn test_detect_plain_text() {
        let plain = "Just some plain text\nwith multiple lines\nnothing special";
        assert_eq!(detect_content_type(plain), ContentType::PlainText);
    }

    #[test]
    fn test_markdown_xss_prevention() {
        let malicious = "# Test\n<script>alert('xss')</script>";
        let rendered = render_markdown(malicious);
        assert!(!rendered.contains("<script>"));
    }

    #[test]
    fn test_markdown_table() {
        let md = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1 | Cell 2 |";
        let rendered = render_markdown(md);
        assert!(rendered.contains("<table>"));
        assert!(rendered.contains("<th>"));
    }

    #[test]
    fn test_html_iframe_escape() {
        let html = r#"<div class="test">Hello "world"</div>"#;
        let escaped = prepare_html_for_iframe(html);
        assert!(escaped.contains("&quot;"));
    }
}
