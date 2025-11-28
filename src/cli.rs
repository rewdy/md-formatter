use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mdfmt")]
#[command(version = "0.1.0")]
#[command(about = "Fast, opinionated Markdown formatter", long_about = None)]
pub struct Args {
    /// File to format (use - for stdin)
    #[arg(value_name = "PATH")]
    pub path: Option<String>,

    /// Write formatted output to file in-place
    #[arg(short, long)]
    pub write: bool,

    /// Check if file is formatted (exit with 1 if not)
    #[arg(long)]
    pub check: bool,

    /// Read from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Line width for wrapping (default: 80)
    #[arg(long, default_value = "80")]
    pub width: usize,
}

impl Args {
    /// Determine the input path
    pub fn get_input_path(&self) -> Result<InputSource, String> {
        if self.stdin {
            Ok(InputSource::Stdin)
        } else if let Some(path) = &self.path {
            if path == "-" {
                Ok(InputSource::Stdin)
            } else {
                Ok(InputSource::File(PathBuf::from(path)))
            }
        } else {
            Err("No input provided. Use --stdin or specify a file path.".to_string())
        }
    }
}

#[derive(Debug)]
pub enum InputSource {
    File(PathBuf),
    Stdin,
}
