use pulldown_cmark::{CowStr, Event, Tag};
use std::str::FromStr;

/// How to handle prose wrapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapMode {
    /// Wrap prose if it exceeds the print width
    Always,
    /// Un-wrap each block of prose into one line
    Never,
    /// Do nothing, leave prose as-is (default)
    #[default]
    Preserve,
}

impl FromStr for WrapMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            "preserve" => Ok(Self::Preserve),
            _ => Err(format!(
                "Invalid wrap mode: '{}'. Expected: always, never, preserve",
                s
            )),
        }
    }
}

/// Represents an inline element that can be buffered before wrapping
#[derive(Debug, Clone)]
enum InlineElement {
    /// Regular text content
    Text(String),
    /// Inline code (`code`)
    Code(String),
    /// Start of emphasis (*)
    EmphasisStart,
    /// End of emphasis (*)
    EmphasisEnd,
    /// Start of strong (**)
    StrongStart,
    /// End of strong (**)
    StrongEnd,
    /// Start of strikethrough (~~)
    StrikethroughStart,
    /// End of strikethrough (~~)
    StrikethroughEnd,
    /// Start of link ([)
    LinkStart,
    /// End of link with URL](url)
    LinkEnd(String),
    /// Start of image (![)
    ImageStart,
    /// End of image with URL and optional title](url "title")
    ImageEnd { url: String, title: String },
    /// Hard break from source (preserve as `  \n`)
    HardBreak,
    /// Soft break from source (treat as space)
    SoftBreak,
}

/// Context for tracking where we are in the document
#[derive(Debug, Clone, PartialEq)]
pub enum Context {
    Paragraph,
    Heading { level: u32 },
    List { ordered: bool },
    ListItem,
    Blockquote,
    CodeBlock,
    Strong,
    Emphasis,
    Strikethrough,
    Link { url: String },
    Image { url: String, title: String },
}

/// Main formatter struct
pub struct Formatter {
    /// Final output
    output: String,
    /// Target line width
    line_width: usize,
    /// How to handle prose wrapping
    wrap_mode: WrapMode,
    /// Buffer for accumulating inline elements before wrapping
    inline_buffer: Vec<InlineElement>,
    /// Context stack for tracking nesting
    context_stack: Vec<Context>,
    /// Current list nesting depth
    list_depth: usize,
    /// Current blockquote nesting depth
    blockquote_depth: usize,
    /// Are we inside a code block?
    in_code_block: bool,
}

impl Formatter {
    /// Create a new formatter with the given line width and wrap mode
    pub fn new(line_width: usize) -> Self {
        Self::with_wrap_mode(line_width, WrapMode::default())
    }

    /// Create a new formatter with the given line width and wrap mode
    pub fn with_wrap_mode(line_width: usize, wrap_mode: WrapMode) -> Self {
        Self {
            output: String::new(),
            line_width,
            wrap_mode,
            inline_buffer: Vec::new(),
            context_stack: Vec::new(),
            list_depth: 0,
            blockquote_depth: 0,
            in_code_block: false,
        }
    }

    /// Format markdown from a list of events
    pub fn format(&mut self, events: Vec<Event>) -> String {
        for event in events {
            self.process_event(event);
        }

        // Flush any remaining content
        self.flush_inline_buffer();

        // Ensure single trailing newline
        let result = self.output.trim_end().to_string();
        if result.is_empty() {
            result
        } else {
            result + "\n"
        }
    }

    fn process_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start_tag(tag),
            Event::End(tag) => self.handle_end_tag(tag),
            Event::Text(text) => self.handle_text(text),
            Event::Code(code) => self.handle_inline_code(code),
            Event::Html(html) => self.handle_html(html),
            Event::SoftBreak => self.handle_soft_break(),
            Event::HardBreak => self.handle_hard_break(),
            Event::Rule => self.handle_rule(),
            Event::FootnoteReference(_) => {}
            Event::TaskListMarker(checked) => self.handle_task_list_marker(checked),
        }
    }

    /// Get the prefix for the current line (blockquote markers)
    fn get_line_prefix(&self) -> String {
        let mut prefix = String::new();
        for _ in 0..self.blockquote_depth {
            prefix.push_str("> ");
        }
        prefix
    }

    /// Get the continuation indent for wrapped lines
    fn get_continuation_indent(&self) -> String {
        let mut indent = self.get_line_prefix();

        // Add list indentation for continuation lines
        if self.list_depth > 0 {
            // Each list level needs indentation, plus space for the marker
            indent.push_str(&"  ".repeat(self.list_depth));
        }

        indent
    }

    /// Convert inline buffer to a flat string (for wrapping), preserving structure
    fn render_inline_buffer(&self) -> String {
        let mut result = String::new();
        for elem in &self.inline_buffer {
            match elem {
                InlineElement::Text(s) => result.push_str(s),
                InlineElement::Code(s) => {
                    result.push('`');
                    result.push_str(s);
                    result.push('`');
                }
                InlineElement::EmphasisStart => result.push('*'),
                InlineElement::EmphasisEnd => result.push('*'),
                InlineElement::StrongStart => result.push_str("**"),
                InlineElement::StrongEnd => result.push_str("**"),
                InlineElement::StrikethroughStart => result.push_str("~~"),
                InlineElement::StrikethroughEnd => result.push_str("~~"),
                InlineElement::LinkStart => result.push('['),
                InlineElement::LinkEnd(url) => {
                    result.push_str("](");
                    result.push_str(url);
                    result.push(')');
                }
                InlineElement::ImageStart => result.push_str("!["),
                InlineElement::ImageEnd { url, title } => {
                    result.push_str("](");
                    result.push_str(url);
                    if !title.is_empty() {
                        result.push_str(" \"");
                        result.push_str(title);
                        result.push('"');
                    }
                    result.push(')');
                }
                InlineElement::HardBreak => result.push('\u{FFFF}'), // Placeholder for hard break
                InlineElement::SoftBreak => {
                    match self.wrap_mode {
                        WrapMode::Preserve => result.push('\u{FFFE}'), // Placeholder for preserved line break
                        WrapMode::Always | WrapMode::Never => result.push(' '),
                    }
                }
            }
        }
        result
    }

    /// Wrap text to fit within line_width
    /// Returns wrapped text with proper line prefixes
    fn wrap_text(&self, text: &str, first_line_prefix: &str, continuation_prefix: &str) -> String {
        let hard_break_placeholder = "\u{FFFF}";
        let soft_break_placeholder = "\u{FFFE}";

        match self.wrap_mode {
            WrapMode::Preserve => {
                // Preserve mode: keep line breaks as-is, just add prefixes
                self.wrap_text_preserve(
                    text,
                    first_line_prefix,
                    continuation_prefix,
                    hard_break_placeholder,
                    soft_break_placeholder,
                )
            }
            WrapMode::Never => {
                // Never mode: unwrap everything to single lines (per paragraph)
                self.wrap_text_never(text, first_line_prefix, hard_break_placeholder)
            }
            WrapMode::Always => {
                // Always mode: reflow text to fit width
                self.wrap_text_always(
                    text,
                    first_line_prefix,
                    continuation_prefix,
                    hard_break_placeholder,
                )
            }
        }
    }

    /// Preserve mode: keep original line breaks
    fn wrap_text_preserve(
        &self,
        text: &str,
        first_line_prefix: &str,
        continuation_prefix: &str,
        hard_break_placeholder: &str,
        soft_break_placeholder: &str,
    ) -> String {
        let mut result = String::new();
        let mut is_first_line = true;

        // Split on both hard and soft break placeholders
        // We need to track which type of break it was
        let mut remaining = text;

        while !remaining.is_empty() {
            // Find the next break (either hard or soft)
            let hard_pos = remaining.find(hard_break_placeholder);
            let soft_pos = remaining.find(soft_break_placeholder);

            let (segment, break_type, rest) = match (hard_pos, soft_pos) {
                (Some(h), Some(s)) if h < s => {
                    let (seg, rest) = remaining.split_at(h);
                    (seg, Some("hard"), &rest[hard_break_placeholder.len()..])
                }
                (Some(h), Some(s)) if s < h => {
                    let (seg, rest) = remaining.split_at(s);
                    (seg, Some("soft"), &rest[soft_break_placeholder.len()..])
                }
                (Some(h), None) => {
                    let (seg, rest) = remaining.split_at(h);
                    (seg, Some("hard"), &rest[hard_break_placeholder.len()..])
                }
                (None, Some(s)) => {
                    let (seg, rest) = remaining.split_at(s);
                    (seg, Some("soft"), &rest[soft_break_placeholder.len()..])
                }
                (Some(h), Some(_)) => {
                    // h == s, shouldn't happen, but handle it
                    let (seg, rest) = remaining.split_at(h);
                    (seg, Some("hard"), &rest[hard_break_placeholder.len()..])
                }
                (None, None) => (remaining, None, ""),
            };

            // Add the prefix
            let prefix = if is_first_line {
                first_line_prefix
            } else {
                continuation_prefix
            };
            result.push_str(prefix);

            // Add the segment content (normalize internal whitespace but preserve words)
            let words: Vec<&str> = segment.split_whitespace().collect();
            result.push_str(&words.join(" "));

            // Add the appropriate line ending
            match break_type {
                Some("hard") => {
                    result.push_str("  \n");
                }
                Some("soft") => {
                    result.push('\n');
                }
                None => {}
                _ => {}
            }

            remaining = rest;
            is_first_line = false;
        }

        result
    }

    /// Never mode: unwrap to single line
    fn wrap_text_never(
        &self,
        text: &str,
        first_line_prefix: &str,
        hard_break_placeholder: &str,
    ) -> String {
        // Split on hard breaks - those we preserve
        let segments: Vec<&str> = text.split(hard_break_placeholder).collect();
        let mut result = String::new();

        for (seg_idx, segment) in segments.iter().enumerate() {
            let words: Vec<&str> = segment.split_whitespace().collect();

            if seg_idx == 0 {
                result.push_str(first_line_prefix);
            }

            result.push_str(&words.join(" "));

            // Add hard break if not the last segment
            if seg_idx < segments.len() - 1 {
                result.push_str("  \n");
                result.push_str(first_line_prefix);
            }
        }

        result
    }

    /// Always mode: reflow text to fit width (original behavior)
    fn wrap_text_always(
        &self,
        text: &str,
        first_line_prefix: &str,
        continuation_prefix: &str,
        hard_break_placeholder: &str,
    ) -> String {
        // First, handle hard breaks by splitting on them
        let segments: Vec<&str> = text.split(hard_break_placeholder).collect();

        let mut result = String::new();

        for (seg_idx, segment) in segments.iter().enumerate() {
            // Normalize whitespace within this segment
            let words: Vec<&str> = segment.split_whitespace().collect();

            if words.is_empty() {
                if seg_idx < segments.len() - 1 {
                    // There was a hard break here, add it
                    if !result.is_empty() {
                        result.push_str("  \n");
                        result.push_str(continuation_prefix);
                    }
                }
                continue;
            }

            let prefix = if seg_idx == 0 && result.is_empty() {
                first_line_prefix
            } else {
                continuation_prefix
            };

            let mut current_line = if result.is_empty() || result.ends_with('\n') {
                prefix.to_string()
            } else {
                String::new()
            };

            let mut first_word_on_line = result.is_empty() || result.ends_with('\n');

            for word in &words {
                let space_needed = if first_word_on_line { 0 } else { 1 };
                let would_be_length = current_line.len() + space_needed + word.len();

                if !first_word_on_line && would_be_length > self.line_width {
                    // Wrap to new line (use plain \n - NOT hard break)
                    result.push_str(&current_line);
                    result.push('\n');
                    current_line = continuation_prefix.to_string();
                    current_line.push_str(word);
                    first_word_on_line = false;
                } else {
                    if !first_word_on_line {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                    first_word_on_line = false;
                }
            }

            result.push_str(&current_line);

            // Add hard break if not the last segment
            if seg_idx < segments.len() - 1 {
                result.push_str("  \n");
                result.push_str(continuation_prefix);
            }
        }

        result
    }

    /// Flush the inline buffer, wrapping text appropriately
    fn flush_inline_buffer(&mut self) {
        if self.inline_buffer.is_empty() {
            return;
        }

        let rendered = self.render_inline_buffer();

        if rendered.trim().is_empty() {
            self.inline_buffer.clear();
            return;
        }

        let prefix = self.get_line_prefix();
        let continuation = self.get_continuation_indent();

        let wrapped = self.wrap_text(&rendered, &prefix, &continuation);
        self.output.push_str(&wrapped);
        self.inline_buffer.clear();
    }

    /// Ensure there's a blank line before the next block element
    fn ensure_blank_line(&mut self) {
        if self.output.is_empty() {
            return;
        }
        if !self.output.ends_with("\n\n") {
            if self.output.ends_with('\n') {
                self.output.push('\n');
            } else {
                self.output.push_str("\n\n");
            }
        }
    }

    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading(level, _, _) => {
                self.flush_inline_buffer();
                self.ensure_blank_line();
                let level_num = level as usize;
                self.output.push_str(&"#".repeat(level_num));
                self.output.push(' ');
                self.context_stack.push(Context::Heading {
                    level: level_num as u32,
                });
            }

            Tag::Paragraph => {
                self.flush_inline_buffer();
                // Don't add blank line if we're directly inside a list item
                // (list items implicitly contain paragraphs)
                let in_list_item = self.context_stack.last() == Some(&Context::ListItem);
                if !in_list_item {
                    self.ensure_blank_line();
                }
                // Don't add prefix here - wrap_text will handle it
                self.context_stack.push(Context::Paragraph);
            }

            Tag::List(first_item_number) => {
                self.flush_inline_buffer();
                self.ensure_blank_line();
                self.list_depth += 1;
                self.context_stack.push(Context::List {
                    ordered: first_item_number.is_some(),
                });
            }

            Tag::Item => {
                self.flush_inline_buffer();
                if !self.output.ends_with('\n') && !self.output.is_empty() {
                    self.output.push('\n');
                }

                // Add blockquote prefix
                let prefix = self.get_line_prefix();
                self.output.push_str(&prefix);

                // Add list indentation (for nested lists)
                if self.list_depth > 1 {
                    self.output.push_str(&"  ".repeat(self.list_depth - 1));
                }

                // Add list marker
                let is_ordered = self
                    .context_stack
                    .iter()
                    .rev()
                    .find_map(|c| match c {
                        Context::List { ordered, .. } => Some(*ordered),
                        _ => None,
                    })
                    .unwrap_or(false);

                if is_ordered {
                    self.output.push_str("1. ");
                } else {
                    self.output.push_str("- ");
                }

                self.context_stack.push(Context::ListItem);
            }

            Tag::BlockQuote => {
                self.flush_inline_buffer();
                self.ensure_blank_line();
                self.blockquote_depth += 1;
                self.context_stack.push(Context::Blockquote);
            }

            Tag::CodeBlock(kind) => {
                self.flush_inline_buffer();
                self.ensure_blank_line();
                self.in_code_block = true;

                // Extract language if specified
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                        lang.to_string()
                    }
                    _ => String::new(),
                };

                self.output.push_str("```");
                self.output.push_str(&lang);
                self.output.push('\n');
                self.context_stack.push(Context::CodeBlock);
            }

            Tag::Strong => {
                self.inline_buffer.push(InlineElement::StrongStart);
                self.context_stack.push(Context::Strong);
            }

            Tag::Emphasis => {
                self.inline_buffer.push(InlineElement::EmphasisStart);
                self.context_stack.push(Context::Emphasis);
            }

            Tag::Strikethrough => {
                self.inline_buffer.push(InlineElement::StrikethroughStart);
                self.context_stack.push(Context::Strikethrough);
            }

            Tag::Link(_, url, _) => {
                self.inline_buffer.push(InlineElement::LinkStart);
                self.context_stack.push(Context::Link {
                    url: url.to_string(),
                });
            }

            Tag::Image(_, url, title) => {
                self.inline_buffer.push(InlineElement::ImageStart);
                self.context_stack.push(Context::Image {
                    url: url.to_string(),
                    title: title.to_string(),
                });
            }

            _ => {}
        }
    }

    fn handle_end_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { .. } => {
                self.flush_inline_buffer();
                self.output.push('\n');
                self.context_stack.pop();
            }

            Tag::Paragraph => {
                self.flush_inline_buffer();
                self.output.push('\n');
                self.context_stack.pop();
            }

            Tag::List(_) => {
                self.flush_inline_buffer();
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                self.list_depth = self.list_depth.saturating_sub(1);
                self.context_stack.pop();
            }

            Tag::Item => {
                self.flush_inline_buffer();
                self.context_stack.pop();
            }

            Tag::BlockQuote => {
                self.flush_inline_buffer();
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
                self.context_stack.pop();
            }

            Tag::CodeBlock(_) => {
                self.output.push_str("```\n");
                self.in_code_block = false;
                self.context_stack.pop();
            }

            Tag::Strong => {
                self.inline_buffer.push(InlineElement::StrongEnd);
                self.context_stack.pop();
            }

            Tag::Emphasis => {
                self.inline_buffer.push(InlineElement::EmphasisEnd);
                self.context_stack.pop();
            }

            Tag::Strikethrough => {
                self.inline_buffer.push(InlineElement::StrikethroughEnd);
                self.context_stack.pop();
            }

            Tag::Link(_, _, _) => {
                // Get the URL from context
                if let Some(Context::Link { url }) = self.context_stack.pop() {
                    self.inline_buffer.push(InlineElement::LinkEnd(url));
                }
            }

            Tag::Image(_, _, _) => {
                // Get the URL and title from context
                if let Some(Context::Image { url, title }) = self.context_stack.pop() {
                    self.inline_buffer
                        .push(InlineElement::ImageEnd { url, title });
                }
            }

            _ => {}
        }
    }

    fn handle_text(&mut self, text: CowStr) {
        if self.in_code_block {
            // Code blocks: preserve exactly
            self.output.push_str(&text);
        } else {
            // Regular text: add to inline buffer
            self.inline_buffer
                .push(InlineElement::Text(text.to_string()));
        }
    }

    fn handle_inline_code(&mut self, code: CowStr) {
        self.inline_buffer
            .push(InlineElement::Code(code.to_string()));
    }

    fn handle_html(&mut self, html: CowStr) {
        self.flush_inline_buffer();
        self.ensure_blank_line();
        self.output.push_str(&html);
        if !html.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn handle_soft_break(&mut self) {
        if !self.in_code_block {
            // Soft break = space (will be normalized during flush)
            self.inline_buffer.push(InlineElement::SoftBreak);
        }
    }

    fn handle_hard_break(&mut self) {
        // Hard break from source - preserve it!
        self.inline_buffer.push(InlineElement::HardBreak);
    }

    fn handle_rule(&mut self) {
        self.flush_inline_buffer();
        self.ensure_blank_line();
        self.output.push_str("---\n");
    }

    fn handle_task_list_marker(&mut self, checked: bool) {
        if checked {
            self.inline_buffer
                .push(InlineElement::Text("[x] ".to_string()));
        } else {
            self.inline_buffer
                .push(InlineElement::Text("[ ] ".to_string()));
        }
    }
}
