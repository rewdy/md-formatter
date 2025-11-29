//! NAPI bindings for md-formatter
//!
//! This module exposes the Rust markdown formatter to Node.js via NAPI-RS.

use napi_derive::napi;

use crate::{extract_frontmatter, parse_markdown, Formatter, WrapMode};

/// Options for the markdown formatter
#[napi(object)]
pub struct FormatOptions {
    /// Target line width for wrapping (default: 80)
    pub width: Option<u32>,
    /// How to wrap prose: "always", "never", or "preserve" (default: "preserve")
    pub wrap: Option<String>,
}

/// Result of a format operation
#[napi(object)]
pub struct FormatResult {
    /// The formatted markdown content
    pub content: String,
    /// Whether the content was changed
    pub changed: bool,
}

fn parse_wrap_mode(wrap: Option<String>) -> WrapMode {
    wrap.and_then(|s| WrapMode::from_str(&s))
        .unwrap_or_default()
}

/// Format a markdown string with the given options.
///
/// @param input - The markdown string to format
/// @param options - Optional formatting options
/// @returns The formatted markdown string
#[napi]
pub fn format_markdown(input: String, options: Option<FormatOptions>) -> String {
    let width = options.as_ref().and_then(|o| o.width).unwrap_or(80) as usize;
    let wrap_mode = parse_wrap_mode(options.and_then(|o| o.wrap));

    let (frontmatter, content) = extract_frontmatter(&input);
    let events = parse_markdown(content);
    let mut formatter = Formatter::with_wrap_mode(width, wrap_mode);
    let formatted = formatter.format(events);

    if let Some(fm) = frontmatter {
        fm + &formatted
    } else {
        formatted
    }
}

/// Format a markdown string and return both the result and whether it changed.
///
/// @param input - The markdown string to format
/// @param options - Optional formatting options
/// @returns An object with `content` (formatted string) and `changed` (boolean)
#[napi]
pub fn format_markdown_with_result(input: String, options: Option<FormatOptions>) -> FormatResult {
    let formatted = format_markdown(input.clone(), options);
    let changed = formatted != input;
    FormatResult {
        content: formatted,
        changed,
    }
}

/// Check if a markdown string is already properly formatted.
///
/// @param input - The markdown string to check
/// @param options - Optional formatting options
/// @returns true if the content is already formatted, false otherwise
#[napi]
pub fn check_markdown(input: String, options: Option<FormatOptions>) -> bool {
    let formatted = format_markdown(input.clone(), options);
    formatted == input
}
