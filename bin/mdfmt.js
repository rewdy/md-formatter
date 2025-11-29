#!/usr/bin/env node

const { formatMarkdown, formatFiles, checkFiles } = require("../index.js");

const args = process.argv.slice(2);

// Parse arguments
let width = 80;
let wrap = "preserve";
let orderedList = "ascending";
let check = false;
let write = false;
let stdin = false;
let patterns = [];
let exclude = [];
let noDefaultExcludes = false;

function printHelp() {
  console.log(`mdfmt - A fast, opinionated Markdown formatter

Usage: mdfmt [OPTIONS] [FILES...]

Arguments:
  [FILES...]  Markdown files, directories, or glob patterns to format

Options:
  -w, --write              Write formatted output back to file
  -c, --check              Check if files are formatted (exit 1 if not)
      --stdin              Read from standard input
      --width <NUMBER>     Maximum line width [default: 80]
      --wrap <MODE>        Prose wrapping: always, never, preserve [default: preserve]
      --ordered-list <MODE> Ordered list style: ascending, one [default: ascending]
      --exclude <DIR>      Additional directories to exclude (can be used multiple times)
      --no-default-excludes Don't exclude node_modules, .git, etc. by default
  -h, --help               Print help
  -V, --version            Print version`);
}

function printVersion() {
  const pkg = require("../package.json");
  console.log(`mdfmt ${pkg.version}`);
}

// Parse CLI arguments
for (let i = 0; i < args.length; i++) {
  const arg = args[i];

  if (arg === "-h" || arg === "--help") {
    printHelp();
    process.exit(0);
  } else if (arg === "-V" || arg === "--version") {
    printVersion();
    process.exit(0);
  } else if (arg === "-w" || arg === "--write") {
    write = true;
  } else if (arg === "-c" || arg === "--check") {
    check = true;
  } else if (arg === "--stdin") {
    stdin = true;
  } else if (arg === "--width") {
    width = parseInt(args[++i], 10);
    if (isNaN(width)) {
      console.error("Error: --width requires a number");
      process.exit(2);
    }
  } else if (arg === "--wrap") {
    wrap = args[++i];
    if (!["always", "never", "preserve"].includes(wrap)) {
      console.error("Error: --wrap must be always, never, or preserve");
      process.exit(2);
    }
  } else if (arg === "--ordered-list") {
    orderedList = args[++i];
    if (!["ascending", "one"].includes(orderedList)) {
      console.error("Error: --ordered-list must be ascending or one");
      process.exit(2);
    }
  } else if (arg === "--exclude") {
    exclude.push(args[++i]);
  } else if (arg === "--no-default-excludes") {
    noDefaultExcludes = true;
  } else if (arg.startsWith("-")) {
    console.error(`Error: Unknown option: ${arg}`);
    process.exit(2);
  } else {
    patterns.push(arg);
  }
}

const options = {
  width,
  wrap,
  orderedList,
  exclude: exclude.length > 0 ? exclude : undefined,
  noDefaultExcludes: noDefaultExcludes || undefined,
};

async function readStdin() {
  return new Promise((resolve) => {
    let data = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => (data += chunk));
    process.stdin.on("end", () => resolve(data));
  });
}

async function main() {
  // Handle stdin
  if (stdin) {
    const input = await readStdin();
    const output = formatMarkdown(input, options);
    process.stdout.write(output);
    process.exit(0);
  }

  // Require patterns if not using stdin
  if (patterns.length === 0) {
    printHelp();
    process.exit(2);
  }

  let hasErrors = false;
  let needsFormatting = false;

  if (check) {
    // Check mode - use Rust's checkFiles
    const results = checkFiles(patterns, options);

    if (results.length === 0) {
      console.error("No markdown files found.");
      process.exit(2);
    }

    for (const result of results) {
      if (result.error) {
        console.error(`Error: ${result.path}: ${result.error}`);
        hasErrors = true;
      } else if (result.changed) {
        console.log(`Would reformat: ${result.path}`);
        needsFormatting = true;
      }
    }

    const wouldChange = results.filter((r) => r.changed && !r.error).length;
    const total = results.filter((r) => !r.error).length;

    if (wouldChange > 0) {
      console.error(`${wouldChange} file(s) would be reformatted`);
    } else {
      console.error(`All ${total} file(s) are formatted correctly`);
    }
  } else if (write) {
    // Write mode - use Rust's formatFiles
    const results = formatFiles(patterns, options);

    if (results.length === 0) {
      console.error("No markdown files found.");
      process.exit(2);
    }

    for (const result of results) {
      if (result.error) {
        console.error(`Error: ${result.path}: ${result.error}`);
        hasErrors = true;
      } else if (result.changed) {
        console.log(`Formatted: ${result.path}`);
      }
    }
  } else {
    // Output to stdout - only works with single file
    if (patterns.length > 1) {
      console.error(
        "Error: Cannot output multiple files to stdout. Use --write to format in-place."
      );
      process.exit(2);
    }

    // For stdout mode, we still use checkFiles to resolve patterns,
    // but then read and format manually
    const results = checkFiles(patterns, options);

    if (results.length === 0) {
      console.error("No markdown files found.");
      process.exit(2);
    }

    if (results.length > 1) {
      console.error(
        "Error: Pattern matches multiple files. Use --write to format in-place."
      );
      process.exit(2);
    }

    const result = results[0];
    if (result.error) {
      console.error(`Error: ${result.path}: ${result.error}`);
      process.exit(2);
    }

    // Read and format the single file
    const fs = require("fs");
    const content = fs.readFileSync(result.path, "utf8");
    const formatted = formatMarkdown(content, options);
    process.stdout.write(formatted);
  }

  if (hasErrors) {
    process.exit(2);
  } else if (check && needsFormatting) {
    process.exit(1);
  } else {
    process.exit(0);
  }
}

main();
