use clap::{Parser, ValueEnum};
use glob::glob;
use std::path::PathBuf;

/// How to handle prose wrapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum WrapMode {
    /// Wrap prose if it exceeds the print width
    Always,
    /// Un-wrap each block of prose into one line
    Never,
    /// Do nothing, leave prose as-is (default)
    #[default]
    Preserve,
}

impl From<WrapMode> for crate::formatter::WrapMode {
    fn from(mode: WrapMode) -> Self {
        match mode {
            WrapMode::Always => Self::Always,
            WrapMode::Never => Self::Never,
            WrapMode::Preserve => Self::Preserve,
        }
    }
}

/// How to handle ordered list numbering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OrderedListMode {
    /// Renumber items sequentially (1, 2, 3, ...) - default
    #[default]
    Ascending,
    /// Use 1. for all items
    One,
}

impl From<OrderedListMode> for crate::formatter::OrderedListMode {
    fn from(mode: OrderedListMode) -> Self {
        match mode {
            OrderedListMode::Ascending => Self::Ascending,
            OrderedListMode::One => Self::One,
        }
    }
}

/// Default directories to exclude when searching
const DEFAULT_EXCLUDES: &[&str] = &["node_modules", "target", ".git", "vendor", "dist", "build"];

#[derive(Parser, Debug)]
#[command(name = "mdfmt")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Fast, opinionated Markdown formatter", long_about = None)]
pub struct Args {
    /// Files or directories to format (supports glob patterns, use - for stdin)
    #[arg(value_name = "PATH")]
    pub paths: Vec<String>,

    /// Write formatted output to file in-place
    #[arg(short, long)]
    pub write: bool,

    /// Check if files are formatted (exit with 1 if not)
    #[arg(long)]
    pub check: bool,

    /// Read from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Line width for wrapping (default: 80)
    #[arg(long, default_value = "80")]
    pub width: usize,

    /// How to wrap prose: always (reflow to width), never (one line per paragraph), preserve (keep as-is)
    #[arg(long, value_enum, default_value = "preserve")]
    pub wrap: WrapMode,

    /// How to number ordered lists: ascending (1, 2, 3), one (all 1.)
    #[arg(long = "ordered-list", value_enum, default_value = "ascending")]
    pub ordered_list: OrderedListMode,

    /// Additional directories to exclude (node_modules, target, .git, vendor, dist, build are excluded by default)
    #[arg(long = "exclude", value_name = "DIR")]
    pub excludes: Vec<String>,

    /// Don't exclude any directories by default
    #[arg(long)]
    pub no_default_excludes: bool,
}

impl Args {
    /// Get the list of directories to exclude
    fn get_excludes(&self) -> Vec<String> {
        let mut excludes: Vec<String> = if self.no_default_excludes {
            Vec::new()
        } else {
            DEFAULT_EXCLUDES.iter().map(|s| s.to_string()).collect()
        };
        excludes.extend(self.excludes.clone());
        excludes
    }

    /// Check if a path should be excluded
    fn should_exclude(&self, path: &std::path::Path, excludes: &[String]) -> bool {
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

    /// Resolve input paths to a list of markdown files or stdin
    pub fn get_input_sources(&self) -> Result<Vec<InputSource>, String> {
        if self.stdin || (self.paths.len() == 1 && self.paths[0] == "-") {
            return Ok(vec![InputSource::Stdin]);
        }

        if self.paths.is_empty() {
            return Err("No input provided. Use --stdin or specify file paths.".to_string());
        }

        let excludes = self.get_excludes();
        let mut sources = Vec::new();

        for pattern in &self.paths {
            let path = PathBuf::from(pattern);

            if path.is_dir() {
                // If it's a directory, find all .md files recursively
                let glob_pattern = format!("{}/**/*.md", pattern);
                self.collect_markdown_files(&glob_pattern, &mut sources, &excludes)?;
            } else if path.is_file() {
                // Single file - must be .md
                if Self::is_markdown_file(&path) {
                    sources.push(InputSource::File(path));
                } else {
                    return Err(format!(
                        "File '{}' is not a markdown file (.md)",
                        path.display()
                    ));
                }
            } else {
                // Treat as glob pattern
                self.collect_markdown_files(pattern, &mut sources, &excludes)?;
            }
        }

        if sources.is_empty() {
            return Err("No markdown files found.".to_string());
        }

        Ok(sources)
    }

    fn collect_markdown_files(
        &self,
        pattern: &str,
        sources: &mut Vec<InputSource>,
        excludes: &[String],
    ) -> Result<(), String> {
        let entries =
            glob(pattern).map_err(|e| format!("Invalid glob pattern '{}': {}", pattern, e))?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    if path.is_file()
                        && Self::is_markdown_file(&path)
                        && !self.should_exclude(&path, excludes)
                    {
                        sources.push(InputSource::File(path));
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not read path: {}", e);
                }
            }
        }

        Ok(())
    }

    fn is_markdown_file(path: &std::path::Path) -> bool {
        path.extension()
            .map(|ext| ext.to_string_lossy().to_lowercase() == "md")
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub enum InputSource {
    File(PathBuf),
    Stdin,
}
