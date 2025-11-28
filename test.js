const {
  formatMarkdown,
  formatMarkdownWithResult,
  checkMarkdown,
} = require("./index.js");

// Test basic formatting
const input = `#  Heading with extra space

This is a paragraph with  multiple   spaces.

- item 1
- item 2
`;

console.log("Testing formatMarkdown...");
const formatted = formatMarkdown(input);
console.log("Input:");
console.log(input);
console.log("Output:");
console.log(formatted);

// Test formatMarkdownWithResult
console.log("\nTesting formatMarkdownWithResult...");
const result = formatMarkdownWithResult(input);
console.log("Changed:", result.changed);

// Test checkMarkdown
console.log("\nTesting checkMarkdown...");
console.log("Is input formatted?", checkMarkdown(input));
console.log("Is output formatted?", checkMarkdown(formatted));

// Test with custom width
console.log("\nTesting with custom width...");
const longLine =
  "This is a very long line that should be wrapped because it exceeds the default line width limit.";
const wrapped = formatMarkdown(longLine, { width: 40 });
console.log("With width 40:");
console.log(wrapped);

// Test idempotence
console.log("\nTesting idempotence...");
const secondPass = formatMarkdown(formatted);
if (formatted === secondPass) {
  console.log("✓ Formatter is idempotent!");
} else {
  console.error("✗ Formatter is NOT idempotent!");
  process.exit(1);
}

console.log("\n✓ All tests passed!");
