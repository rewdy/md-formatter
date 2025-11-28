# Fast, Opinionated Markdown Formatter — Project Plan (Rust)

## 1. Project Overview

Build a **fast**, **opinionated**, **CommonMark/GFM‑focused** Markdown  
formatter written in **Rust**, designed as a clean alternative to Prettier for  
Markdown-only workflows.  
This formatter intentionally omits MDX, embedded language formatting, and  
semantic width analysis, focusing on speed and predictable output.

## 2. High-Level Goals

- Extremely fast formatting (Rust-native performance)
- Deterministic, idempotent output
- Opinionated formatting decisions (no configuration except width)
- Safe round-tripping (no semantic changes)
- Integrates cleanly into Biome-style lint/format pipelines

## 3. Out-of-Scope (Explicitly)

- MDX (JSX/TSX in markdown)
- Formatting embedded code blocks
- Rewriting HTML inside markdown
- Advanced semantic wrapping heuristics

## 4. Core Architecture

### 4.1 Pipeline Overview

```
Raw Markdown
   → Tokenizer (pulldown-cmark)
   → Parsed Events / AST
   → Normalization Pass (custom)
   → Pretty-Printer / Renderer
   → Final Markdown Output
```

### 4.2 Parsing Layer

Use:

- `pulldown-cmark` for parsing
- `pulldown-cmark-to-cmark` OR a custom serializer baseline

Why:

- Fast
- Mature
- Handles GFM reasonably well
- Good balance between lossless-enough events and performance

### 4.3 Internal Representation Strategy

Use one of these two approaches (your implementer can choose):

**A. Event Stream Normalization (Recommended for MVP)**

- 

## Consume `pulldown_cmark::Event` stream

Track context using a small stack:

- List depth
- Blockquote depth
- Code block state
- Paragraph state
- 

Output normalized markdown progressively

This avoids building a full AST and keeps performance maximized.

**B. Build a Full AST**

- More control
- Slower MVP
- Higher complexity

Recommend **A** for initial version.

---

## 5. Formatting Rules (Opinionated Spec)

### 5.1 Paragraphs

- Reflow text into lines of fixed max width (default 80)
- Preserve hard line breaks (`two spaces + newline`)
- Preserve HTML blocks entirely (no formatting inside; treat as opaque)

### 5.2 Headings

- 

Normalize ATX headings:

- `# Heading`
- No trailing `#`
- Ensure a single space after the hash group

### 5.3 Lists

- 

## Normalize ordered lists to always use `1.` style

## Normalize unordered lists to `-` (not `*` or `+`)

Indentation:

- 2 spaces per nesting level
- 

Ensure blank line before and after lists unless tight list rules apply

### 5.4 Blockquotes

- Each line begins with correct number of `>` prefixes
- Reflow text *inside* blockquotes (preserve code blocks)
- Blank lines inside blockquotes get the same prefix rules

### 5.5 Code Blocks

- Fenced only (`````)
- Preserve language tag if present
- Do NOT format internal code
- Trim trailing whitespace inside the block but preserve all user content

### 5.6 Inline Elements

- 

## Preserve original emphasis markers where possible (`*italic*` kept as-is)

Normalize link reference format:

- `[text](url)`
- 

Autolinks remain `<http://example.com>`

### 5.7 Horizontal Rules

- Normalize to:

```
---
```

### 5.8 Frontmatter

- If file begins with:

```
---
key: value
---
```

then preserve block exactly without changes.

---

## 6. Pretty Printer Design

### 6.1 Writing Strategy

Use a `Formatter` struct:

```
struct Formatter {
    output: String,
    indent_level: usize,
    line_width: usize,
}
```

Provides helpers:

- `write_line`
- `write_wrapped`
- `write_indent`
- `push_list_marker`
- `push_blockquote_prefix`

### 6.2 Word Wrapping Algorithm

Use a simple greedy wrapper:

```
for each token:
    if adding token exceeds line_width:
        line break
    else:
        append token
```

Special rules:

- Blockquote prefixes are considered part of line width.
- Leading indentation for list items considered part of width.

### 6.3 Handling Context

Maintain a stack:

- `Context::Paragraph`
- `Context::List(Ordered|Unordered, depth)`
- `Context::Blockquote(depth)`
- `Context::CodeBlock(fence_language)`
- `Context::HtmlBlock`

Push/pop as events are consumed.

---

## 7. CLI Design

### 7.1 CLI Requirements

Command:

```
mdfmt [options] <path>
```

Options:

- `--write` (in-place)
- `--check` (verify formatted)
- `--stdin`
- `--width <number>`
- `--version`
- `--help`

Exit codes:

- `0` = formatted or unchanged
- `1` = would change (in check mode)
- `2` = error

### 7.2 Ability to act as formatter for Biome

Support stdin/stdout with deterministic output so Biome can shell out to it.

---

## 8. Testing Strategy

### 8.1 Snapshot Tests

Use `insta` for text snapshots:

- Paragraph wrapping cases
- Nested lists
- Blockquotes
- Mixed inline text
- Heavy edge-case markdown

### 8.2 Round Trip Safety Tests

Ensure:

```
format(format(input)) == format(input)
```

### 8.3 GFM Edge Tests

- Tables (preserve table structure but do not format)
- Autolinks
- Strikethrough

---

## 9. Performance Targets

- Must format a 100 KB markdown file < **5 ms**
- Must format a 1 MB markdown file < **50–100 ms**
- Memory footprint < **5 MB**

Benchmark against:

- Prettier
- mdformat
- dprint

---

## 10. Future Roadmap (Post-MVP)

- Optional MDX support
- Format tables (alignment)
- Format inline HTML
- Embedded code block formatting via plugins
- Config system
- Biome-internal integration with WASM

---

## 11. Deliverables Summary

- Rust crate (`mdfmt`)
- CLI binary
- Core formatter module
- Unit tests + snapshot tests
- Benchmarks

This is a fully scoped plan another LLM can implement directly.
