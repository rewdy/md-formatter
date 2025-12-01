#[cfg(feature = "cli")]
pub mod cli;
pub mod formatter;
pub mod parser;

// Only include NAPI bindings when the napi feature is enabled
#[cfg(feature = "napi")]
pub mod napi;

pub use formatter::{Formatter, OrderedListMode, WrapMode};
pub use parser::{extract_frontmatter, parse_markdown};

#[cfg(test)]
mod tests {
    use crate::{extract_frontmatter, parse_markdown, Formatter, OrderedListMode, WrapMode};

    fn format_markdown(input: &str) -> String {
        let events = parse_markdown(input);
        let mut formatter = Formatter::new(80);
        formatter.format(events)
    }

    fn format_markdown_always(input: &str) -> String {
        let events = parse_markdown(input);
        let mut formatter = Formatter::with_wrap_mode(80, WrapMode::Always);
        formatter.format(events)
    }

    fn format_markdown_one(input: &str) -> String {
        let events = parse_markdown(input);
        let mut formatter = Formatter::with_options(80, WrapMode::default(), OrderedListMode::One);
        formatter.format(events)
    }

    /// Format markdown with frontmatter support
    fn format_markdown_full(input: &str) -> String {
        let (frontmatter, content) = extract_frontmatter(input);
        let events = parse_markdown(content);
        let mut formatter = Formatter::with_wrap_mode(80, WrapMode::Always);
        let formatted = formatter.format(events);

        if let Some(fm) = frontmatter {
            fm + &formatted
        } else {
            formatted
        }
    }

    // ==========================================================
    // Unit Tests
    // ==========================================================

    #[test]
    fn test_heading_normalization() {
        let input = "# Heading 1\n## Heading 2";
        let output = format_markdown(input);
        let expected = "# Heading 1\n\n## Heading 2\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_list_normalization() {
        let input = "- Item 1\n- Item 2\n- Item 3";
        let output = format_markdown(input);
        let expected = "- Item 1\n- Item 2\n- Item 3\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_ordered_list_ascending_mode() {
        // Default mode: items are numbered 1, 2, 3, ...
        let input = "1. First\n1. Second\n1. Third";
        let output = format_markdown(input);
        assert!(output.contains("1. First"));
        assert!(output.contains("2. Second"));
        assert!(output.contains("3. Third"));
    }

    #[test]
    fn test_multiple_ordered_list_ascending_mode() {
        // Default mode: items are numbered 1, 2, 3, ...
        let input = "1. First\n1. Second\n1. Third\n\nHere is one more\n\n1. Another\n1. List";
        let output = format_markdown(input);
        assert!(output.contains("1. First"));
        assert!(output.contains("2. Second"));
        assert!(output.contains("3. Third"));
        assert!(output.contains("1. Another"));
        assert!(output.contains("2. List"));
    }

    #[test]
    fn test_nested_lists_have_no_empty_lines() {
        let input = "- First\n- Second\n  - Subitem one\n  - Subitem two\n- Third";
        let output = format_markdown(input);
        assert!(!output.contains("\n\n  - Subitem one"));
    }

    #[test]
    fn test_ordered_list_one_mode() {
        // One mode: all items use "1."
        let input = "1. First\n2. Second\n3. Third";
        let output = format_markdown_one(input);
        assert!(output.contains("1. First"));
        assert!(output.contains("1. Second"));
        assert!(output.contains("1. Third"));
    }

    #[test]
    fn test_emphasis() {
        let input = "This is *italic* and **bold** text.";
        let output = format_markdown(input);
        let expected = "This is *italic* and **bold** text.\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_code_block() {
        let input = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let output = format_markdown(input);
        assert!(output.contains("```"));
        assert!(output.contains("fn main()"));
    }

    #[test]
    fn test_text_wrapping() {
        let input = "This is a very long line that should probably be wrapped because it exceeds the line width limit that we have set for the formatter.";
        let output = format_markdown_always(input);
        // Check that it was wrapped (has multiple lines)
        assert!(output.lines().count() > 1);
    }

    #[test]
    fn test_idempotence() {
        let input = "# Hello\n\nThis is a paragraph with *emphasis*.\n\n- Item 1\n- Item 2\n";
        let first_pass = format_markdown(input);
        let second_pass = format_markdown(&first_pass);
        assert_eq!(first_pass, second_pass, "Formatter should be idempotent");
    }

    #[test]
    fn test_inline_code() {
        let input = "Use `let x = 5;` for variable declaration.";
        let output = format_markdown(input);
        assert!(output.contains("`let x = 5;`"));
    }

    #[test]
    fn test_horizontal_rule() {
        let input = "Before\n\n---\n\nAfter";
        let output = format_markdown(input);
        assert!(output.contains("---"));
    }

    #[test]
    fn test_nested_lists() {
        let input = "- Item 1\n- Item 2\n  - Nested 1\n  - Nested 2\n- Item 3";
        let output = format_markdown(input);
        assert!(output.contains("  - Nested"));
    }

    #[test]
    fn test_paragraph_wrapping() {
        let input = "This is a short intro paragraph.\n\nThis is another paragraph that is quite long and should be wrapped nicely across multiple lines if needed based on the formatter's width settings.";
        let output = format_markdown(input);
        // Should have two paragraphs separated by blank line
        let parts: Vec<&str> = output.split("\n\n").collect();
        assert!(parts.len() >= 2);
    }

    #[test]
    fn test_blockquote_formatting() {
        let input = "> This is a blockquote\n> with multiple lines";
        let output = format_markdown(input);
        // Should preserve blockquote markers
        assert!(output.contains(">"));
        // Should be idempotent
        let output2 = format_markdown(&output);
        assert_eq!(output, output2);
    }

    #[test]
    fn test_frontmatter_preservation() {
        let input = "---\ntitle: Test\nauthor: Me\n---\n\n# Heading\n\nContent.";
        let (frontmatter, content) = extract_frontmatter(input);

        // Should extract frontmatter
        assert!(frontmatter.is_some());
        assert!(frontmatter.unwrap().contains("title:"));

        // Remaining content should not include frontmatter
        assert!(!content.contains("title:"));
        assert!(content.contains("# Heading"));
    }

    #[test]
    fn test_strikethrough_preservation() {
        let input = "This has ~~strikethrough~~ text.";
        let output = format_markdown(input);
        // Strikethrough should be preserved
        assert!(output.contains("~~strikethrough~~"));
    }

    #[test]
    fn test_gfm_autolinks() {
        let input = "Visit <https://example.com> for more info.";
        let output = format_markdown(input);
        // Should contain link reference
        assert!(output.contains("example.com"));
    }

    #[test]
    fn test_hard_break_preservation() {
        let input = "Line one  \nLine two";
        let output = format_markdown(input);
        // Hard break should be preserved (two spaces before newline)
        assert!(output.contains("  \n"), "Hard break should be preserved");
    }

    #[test]
    fn test_no_spurious_hard_breaks() {
        // A long line that gets wrapped should NOT have hard breaks (when using always mode)
        let input = "This is a very long line that needs to be wrapped because it exceeds eighty characters.";
        let output = format_markdown_always(input);
        // Should not contain hard breaks (two spaces before newline)
        assert!(
            !output.contains("  \n"),
            "Wrapped lines should not have hard breaks"
        );
    }

    // ==========================================================
    // Fixture-Based Tests
    // ==========================================================

    const SIMPLE_GOOD: &str = include_str!("../tests/fixtures/simple-good.md");
    const SIMPLE_BAD: &str = include_str!("../tests/fixtures/simple-bad.md");
    const COMPLEX_GOOD: &str = include_str!("../tests/fixtures/complex-good.md");
    const COMPLEX_BAD: &str = include_str!("../tests/fixtures/complex-bad.md");

    #[test]
    fn test_simple_good_is_idempotent() {
        let formatted = format_markdown(SIMPLE_GOOD);
        let reformatted = format_markdown(&formatted);
        assert_eq!(
            formatted, reformatted,
            "simple-good.md should be idempotent"
        );
    }

    #[test]
    fn test_simple_bad_formats_correctly() {
        let formatted = format_markdown(SIMPLE_BAD);

        // Should be idempotent after formatting
        let reformatted = format_markdown(&formatted);
        assert_eq!(
            formatted, reformatted,
            "Formatted simple-bad.md should be idempotent"
        );

        // Check specific fixes were applied
        assert!(
            formatted.contains("# Simple Document\n"),
            "Heading should have single space"
        );
        assert!(
            formatted.contains("- First item\n"),
            "List items should use dash with single space"
        );
        assert!(
            formatted.contains("1. "),
            "Ordered list should use 1. format"
        );
    }

    #[test]
    fn test_complex_good_is_idempotent() {
        let formatted = format_markdown_full(COMPLEX_GOOD);
        let reformatted = format_markdown_full(&formatted);
        assert_eq!(
            formatted, reformatted,
            "complex-good.md should be idempotent"
        );
    }

    #[test]
    fn test_complex_bad_formats_correctly() {
        let formatted = format_markdown_full(COMPLEX_BAD);

        // Should be idempotent after formatting
        let reformatted = format_markdown_full(&formatted);
        assert_eq!(
            formatted, reformatted,
            "Formatted complex-bad.md should be idempotent"
        );

        // Check frontmatter is preserved
        assert!(
            formatted.starts_with("---\n"),
            "Frontmatter should be preserved"
        );
        assert!(
            formatted.contains("title:"),
            "Frontmatter content should be preserved"
        );

        // Check code blocks are preserved
        assert!(
            formatted.contains("```python"),
            "Python code block should be preserved"
        );
        assert!(
            formatted.contains("def hello_world():"),
            "Code content should be preserved"
        );

        // Check hard breaks are preserved
        assert!(
            formatted.contains("  \n"),
            "Hard breaks should be preserved"
        );
    }

    #[test]
    fn test_complex_preserves_code_blocks() {
        let formatted = format_markdown_full(COMPLEX_GOOD);

        // Code blocks should be completely unchanged
        assert!(formatted.contains("def hello_world():"));
        assert!(formatted.contains("    \"\"\"A simple greeting function.\"\"\""));
        assert!(formatted.contains("const greet = (name) => {"));
    }

    #[test]
    fn test_no_hard_breaks_in_wrapped_output() {
        // Format the bad version with always mode and check that wrapped lines don't have hard breaks
        let formatted = format_markdown_always(SIMPLE_BAD);

        // Count hard breaks (lines ending with two spaces)
        let hard_break_count = formatted
            .lines()
            .filter(|line| line.ends_with("  "))
            .count();

        // simple-bad.md has no intentional hard breaks, so there should be none
        assert_eq!(
            hard_break_count, 0,
            "Wrapped lines should not introduce hard breaks"
        );
    }
}
