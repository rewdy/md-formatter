use pulldown_cmark::{Event, Parser};

/// Extract YAML frontmatter from markdown input if present
/// Returns (frontmatter, remaining_input)
pub fn extract_frontmatter(input: &str) -> (Option<String>, &str) {
    if !input.starts_with("---\n") {
        return (None, input);
    }

    // Find the closing ---
    let after_opening = &input[4..]; // Skip first "---\n"
    if let Some(end_pos) = after_opening.find("\n---\n") {
        let frontmatter = after_opening[..end_pos].to_string();
        let remaining = &after_opening[end_pos + 5..]; // Skip "\n---\n"
                                                       // Include the frontmatter with opening and closing markers, plus blank line
        (Some(format!("---\n{}\n---\n\n", frontmatter)), remaining)
    } else {
        (None, input)
    }
}

/// Parse markdown into events (GFM tables not needed for basic support)
pub fn parse_markdown(input: &str) -> Vec<Event> {
    Parser::new(input).collect()
}
