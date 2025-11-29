use clap::Parser;
use md_formatter::cli::{Args, InputSource};
use md_formatter::{extract_frontmatter, parse_markdown, Formatter};
use std::fs;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let sources = args.get_input_sources()?;
    let mut has_errors = false;
    let mut files_checked = 0;
    let mut files_would_change = 0;

    for source in sources {
        match process_source(&source, &args) {
            Ok(changed) => {
                if args.check {
                    files_checked += 1;
                    if changed {
                        files_would_change += 1;
                        has_errors = true;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                has_errors = true;
            }
        }
    }

    if args.check && files_checked > 0 {
        if files_would_change > 0 {
            eprintln!("{} file(s) would be reformatted", files_would_change);
        } else {
            eprintln!("All {} file(s) are formatted correctly", files_checked);
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

fn process_source(source: &InputSource, args: &Args) -> Result<bool, Box<dyn std::error::Error>> {
    let (content, path_for_output) = match source {
        InputSource::Stdin => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            (buffer, None)
        }
        InputSource::File(path) => {
            let content = fs::read_to_string(path)?;
            (content, Some(path.clone()))
        }
    };

    // Extract frontmatter if present
    let (frontmatter, markdown_content) = extract_frontmatter(&content);

    // Parse and format the markdown content (without frontmatter)
    let events = parse_markdown(markdown_content);
    let mut formatter = Formatter::new(args.width);
    let formatted = formatter.format(events);

    // Prepend frontmatter if it was present
    let final_output = if let Some(fm) = frontmatter {
        fm + &formatted
    } else {
        formatted
    };

    let changed = content != final_output;

    // Output
    if let Some(path) = path_for_output {
        if args.check {
            if changed {
                eprintln!("Would reformat: {}", path.display());
            }
        } else if args.write {
            if changed {
                fs::write(&path, &final_output)?;
                eprintln!("Formatted: {}", path.display());
            }
        } else {
            print!("{}", final_output);
        }
    } else {
        // stdin
        print!("{}", final_output);
    }

    Ok(changed)
}
