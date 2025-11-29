#!/usr/bin/env node

/**
 * Version bump script for md-formatter
 *
 * Updates version in:
 * - Cargo.toml
 * - package.json
 * - npm/[platform]/package.json (all platform packages)
 *
 * Usage:
 *   node scripts/bump-version.mjs patch           (0.1.0 -> 0.1.1)
 *   node scripts/bump-version.mjs minor           (0.1.0 -> 0.2.0)
 *   node scripts/bump-version.mjs major           (0.1.0 -> 1.0.0)
 *   node scripts/bump-version.mjs 1.2.3           (set explicit version)
 *   node scripts/bump-version.mjs patch --dry-run (preview changes)
 */

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

// Parse arguments
const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const bumpType = args.find((a) => !a.startsWith("--"));

if (!bumpType) {
  console.error(
    "Usage: bump-version.mjs <patch|minor|major|x.y.z> [--dry-run]"
  );
  process.exit(1);
}

/**
 * Parse a semver string into components
 */
function parseVersion(version) {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) throw new Error(`Invalid version: ${version}`);
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
  };
}

/**
 * Bump version based on type
 */
function bumpVersion(current, type) {
  // If type is an explicit version, validate and return it
  if (/^\d+\.\d+\.\d+$/.test(type)) {
    return type;
  }

  const v = parseVersion(current);

  switch (type) {
    case "major":
      return `${v.major + 1}.0.0`;
    case "minor":
      return `${v.major}.${v.minor + 1}.0`;
    case "patch":
      return `${v.major}.${v.minor}.${v.patch + 1}`;
    default:
      throw new Error(
        `Invalid bump type: ${type}. Use patch, minor, major, or x.y.z`
      );
  }
}

/**
 * Get current version from package.json
 */
function getCurrentVersion() {
  const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf-8"));
  return pkg.version;
}

/**
 * Update version in a JSON file
 */
function updateJsonFile(filePath, newVersion, changes) {
  const content = readFileSync(filePath, "utf-8");
  const pkg = JSON.parse(content);
  const oldVersion = pkg.version;

  if (oldVersion === newVersion) {
    return false;
  }

  pkg.version = newVersion;

  changes.push({
    file: filePath.replace(ROOT + "/", ""),
    old: oldVersion,
    new: newVersion,
  });

  if (!dryRun) {
    writeFileSync(filePath, JSON.stringify(pkg, null, 2) + "\n");
  }

  return true;
}

/**
 * Update version in Cargo.toml
 */
function updateCargoToml(newVersion, changes) {
  const filePath = join(ROOT, "Cargo.toml");
  const content = readFileSync(filePath, "utf-8");

  const versionMatch = content.match(/^version\s*=\s*"([^"]+)"/m);
  if (!versionMatch) {
    throw new Error("Could not find version in Cargo.toml");
  }

  const oldVersion = versionMatch[1];
  if (oldVersion === newVersion) {
    return false;
  }

  const newContent = content.replace(
    /^(version\s*=\s*)"[^"]+"/m,
    `$1"${newVersion}"`
  );

  changes.push({
    file: "Cargo.toml",
    old: oldVersion,
    new: newVersion,
  });

  if (!dryRun) {
    writeFileSync(filePath, newContent);
  }

  return true;
}

// Main
const currentVersion = getCurrentVersion();
const newVersion = bumpVersion(currentVersion, bumpType);

console.log(`\n📦 Version bump: ${currentVersion} → ${newVersion}\n`);

if (dryRun) {
  console.log("🔍 DRY RUN - no files will be modified\n");
}

const changes = [];

// Update Cargo.toml
updateCargoToml(newVersion, changes);

// Update root package.json
updateJsonFile(join(ROOT, "package.json"), newVersion, changes);

// Update all npm platform packages
const npmDir = join(ROOT, "npm");
const platforms = readdirSync(npmDir);

for (const platform of platforms) {
  const pkgPath = join(npmDir, platform, "package.json");
  try {
    updateJsonFile(pkgPath, newVersion, changes);
  } catch (e) {
    // Skip if package.json doesn't exist
  }
}

// Print summary
if (changes.length === 0) {
  console.log("No files needed updating.");
} else {
  console.log("Files updated:");
  for (const change of changes) {
    const status = dryRun ? "would update" : "updated";
    console.log(`  ✓ ${change.file}: ${change.old} → ${change.new}`);
  }
  console.log(
    `\nTotal: ${changes.length} file(s) ${dryRun ? "would be " : ""}updated`
  );
}

if (!dryRun && changes.length > 0) {
  console.log(`\nNext steps:`);
  console.log(`  git add -A`);
  console.log(`  git commit -m "chore: bump version to ${newVersion}"`);
  console.log(`  git tag v${newVersion}`);
  console.log(`  git push && git push origin v${newVersion}`);
}

console.log("");
