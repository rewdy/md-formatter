//! NAPI bindings for md-formatter
//!
//! This module exposes the Rust markdown formatter to Node.js via NAPI-RS.

use glob::glob;
use napi_derive::napi;
use std::fs;
use std::path::PathBuf;

use crate::{extract_frontmatter, parse_markdown, Formatter, OrderedListMode, WrapMode};

/// Default directories to exclude when searching
const DEFAULT_EXCLUDES: &[&str] = &["node_modules", "target", ".git", "vendor", "dist", "build"];

/// Options for the markdown formatter
#[napi(object)]
pub struct FormatOptions {
    /// Target line width for wrapping (default: 80)
    pub width: Option<u32>,
    /// How to wrap prose: "always", "never", or "preserve" (default: "preserve")
    pub wrap: Option<String>,
    /// How to number ordered lists: "ascending" (1, 2, 3) or "one" (all 1.) (default: "ascending")
    pub ordered_list: Option<String>,
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
    wrap.and_then(|s| s.parse().ok()).unwrap_or_default()
}

fn parse_ordered_list_mode(mode: Option<String>) -> OrderedListMode {
    mode.and_then(|s| s.parse().ok()).unwrap_or_default()
}

/// Format a markdown string with the given options.
///
/// @param input - The markdown string to format
/// @param options - Optional formatting options
/// @returns The formatted markdown string
#[napi]
pub fn format_markdown(input: String, options: Option<FormatOptions>) -> String {
    let width = options.as_ref().and_then(|o| o.width).unwrap_or(80) as usize;
    let wrap_mode = parse_wrap_mode(options.as_ref().and_then(|o| o.wrap.clone()));
    let ordered_list_mode =
        parse_ordered_list_mode(options.as_ref().and_then(|o| o.ordered_list.clone()));

    let (frontmatter, content) = extract_frontmatter(&input);
    let events = parse_markdown(content);
    let mut formatter = Formatter::with_options(width, wrap_mode, ordered_list_mode);
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
    let width = options.as_ref().and_then(|o| o.width).unwrap_or(80) as usize;
    let wrap_mode = parse_wrap_mode(options.as_ref().and_then(|o| o.wrap.clone()));
    let ordered_list_mode =
        parse_ordered_list_mode(options.as_ref().and_then(|o| o.ordered_list.clone()));

    let (frontmatter, content) = extract_frontmatter(&input);
    let events = parse_markdown(content);
    let mut formatter = Formatter::with_options(width, wrap_mode, ordered_list_mode);
    let formatted_content = formatter.format(events);

    let formatted = if let Some(fm) = frontmatter {
        fm + &formatted_content
    } else {
        formatted_content
    };

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
    let width = options.as_ref().and_then(|o| o.width).unwrap_or(80) as usize;
    let wrap_mode = parse_wrap_mode(options.as_ref().and_then(|o| o.wrap.clone()));
    let ordered_list_mode =
        parse_ordered_list_mode(options.as_ref().and_then(|o| o.ordered_list.clone()));

    let (frontmatter, content) = extract_frontmatter(&input);
    let events = parse_markdown(content);
    let mut formatter = Formatter::with_options(width, wrap_mode, ordered_list_mode);
    let formatted_content = formatter.format(events);

    let formatted = if let Some(fm) = frontmatter {
        fm + &formatted_content
    } else {
        formatted_content
    };

    formatted == input
}

/// Result of a file format operation
#[napi(object)]
pub struct FileResult {
    /// The file path
    pub path: String,
    /// Whether the file was changed (or would be changed in check mode)
    pub changed: bool,
    /// Error message if the file could not be processed
    pub error: Option<String>,
}

/// Options for file operations
#[napi(object)]
pub struct FileOptions {
    /// Target line width for wrapping (default: 80)
    pub width: Option<u32>,
    /// How to wrap prose: "always", "never", or "preserve" (default: "preserve")
    pub wrap: Option<String>,
    /// How to number ordered lists: "ascending" (1, 2, 3) or "one" (all 1.) (default: "ascending")
    pub ordered_list: Option<String>,
    /// Additional directories to exclude
    pub exclude: Option<Vec<String>>,
    /// Don't exclude any directories by default
    pub no_default_excludes: Option<bool>,
}

fn is_markdown_file(path: &std::path::Path) -> bool {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase() == "md")
        .unwrap_or(false)
}

fn should_exclude(path: &std::path::Path, excludes: &[String]) -> bool {
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            if excludes.iter().any(|e| e == name_str.as_ref()) {
                return true;
            }
        }
    }
    false
}

fn get_excludes(options: &Option<FileOptions>) -> Vec<String> {
    let no_default = options
        .as_ref()
        .and_then(|o| o.no_default_excludes)
        .unwrap_or(false);

    let mut excludes: Vec<String> = if no_default {
        Vec::new()
    } else {
        DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect()
    };

    if let Some(opts) = options {
        if let Some(ref extra) = opts.exclude {
            excludes.extend(extra.clone());
        }
    }

    excludes
}

fn resolve_patterns(patterns: Vec<String>, excludes: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for pattern in patterns {
        let path = PathBuf::from(&pattern);

        if path.is_dir() {
            // If it's a directory, find all .md files recursively
            let glob_pattern = format!("{}/**/*.md", pattern);
            if let Ok(entries) = glob(&glob_pattern) {
                for entry in entries.flatten() {
                    if entry.is_file()
                        && is_markdown_file(&entry)
                        && !should_exclude(&entry, excludes)
                    {
                        files.push(entry);
                    }
                }
            }
        } else if path.is_file() {
            // Single file
            if is_markdown_file(&path) {
                files.push(path);
            }
        } else {
            // Treat as glob pattern
            if let Ok(entries) = glob(&pattern) {
                for entry in entries.flatten() {
                    if entry.is_file()
                        && is_markdown_file(&entry)
                        && !should_exclude(&entry, excludes)
                    {
                        files.push(entry);
                    }
                }
            }
        }
    }

    files
}

fn format_file_content(content: &str, options: &Option<FileOptions>) -> String {
    let width = options.as_ref().and_then(|o| o.width).unwrap_or(80) as usize;
    let wrap_mode = parse_wrap_mode(options.as_ref().and_then(|o| o.wrap.clone()));
    let ordered_list_mode =
        parse_ordered_list_mode(options.as_ref().and_then(|o| o.ordered_list.clone()));

    let (frontmatter, md_content) = extract_frontmatter(content);
    let events = parse_markdown(md_content);
    let mut formatter = Formatter::with_options(width, wrap_mode, ordered_list_mode);
    let formatted = formatter.format(events);

    if let Some(fm) = frontmatter {
        fm + &formatted
    } else {
        formatted
    }
}

/// Format files matching the given patterns and write changes to disk.
///
/// @param patterns - File paths, directories, or glob patterns
/// @param options - Optional formatting and file options
/// @returns Array of results for each file processed
#[napi]
pub fn format_files(patterns: Vec<String>, options: Option<FileOptions>) -> Vec<FileResult> {
    let excludes = get_excludes(&options);
    let files = resolve_patterns(patterns, &excludes);
    let mut results = Vec::new();

    for path in files {
        let path_str = path.display().to_string();

        match fs::read_to_string(&path) {
            Ok(content) => {
                let formatted = format_file_content(&content, &options);
                let changed = formatted != content;

                if changed {
                    if let Err(e) = fs::write(&path, &formatted) {
                        results.push(FileResult {
                            path: path_str,
                            changed: false,
                            error: Some(format!("Failed to write: {}", e)),
                        });
                        continue;
                    }
                }

                results.push(FileResult {
                    path: path_str,
                    changed,
                    error: None,
                });
            }
            Err(e) => {
                results.push(FileResult {
                    path: path_str,
                    changed: false,
                    error: Some(format!("Failed to read: {}", e)),
                });
            }
        }
    }

    results
}

/// Check if files matching the given patterns are formatted correctly.
///
/// @param patterns - File paths, directories, or glob patterns
/// @param options - Optional formatting and file options
/// @returns Array of results for each file checked
#[napi]
pub fn check_files(patterns: Vec<String>, options: Option<FileOptions>) -> Vec<FileResult> {
    let excludes = get_excludes(&options);
    let files = resolve_patterns(patterns, &excludes);
    let mut results = Vec::new();

    for path in files {
        let path_str = path.display().to_string();

        match fs::read_to_string(&path) {
            Ok(content) => {
                let formatted = format_file_content(&content, &options);
                let changed = formatted != content;

                results.push(FileResult {
                    path: path_str,
                    changed,
                    error: None,
                });
            }
            Err(e) => {
                results.push(FileResult {
                    path: path_str,
                    changed: false,
                    error: Some(format!("Failed to read: {}", e)),
                });
            }
        }
    }

    results
}
