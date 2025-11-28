use clap::Parser;
use md_formatter::cli::{Args, InputSource};
use md_formatter::{parse_markdown, extract_frontmatter, Formatter};
use std::fs;
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input_source = args.get_input_path()?;

    // Determine if we need to read from stdin or file
    let (content, path_for_output) = match &input_source {
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

    // Output
    if let Some(path) = path_for_output {
        if args.check {
            if content != final_output {
                eprintln!("File would be reformatted: {}", path.display());
                std::process::exit(1);
            }
        } else if args.write {
            fs::write(&path, &final_output)?;
            println!("Formatted: {}", path.display());
        } else {
            print!("{}", final_output);
        }
    } else {
        // stdin
        print!("{}", final_output);
    }

    Ok(())
}

