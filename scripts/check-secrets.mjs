import { execFileSync } from "node:child_process";
import { readFileSync, statSync } from "node:fs";

const trackedOnly = process.argv.includes("--tracked");
const gitArgs = trackedOnly
  ? ["ls-files", "-z"]
  : ["ls-files", "-co", "--exclude-standard", "-z"];
const files = execFileSync("git", gitArgs, { encoding: "utf8" })
  .split("\0")
  .filter(Boolean);

const maxFileSize = 5 * 1024 * 1024;
const rules = [
  ["private-key", /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/g],
  ["provider-api-key", /\bsk-[A-Za-z0-9_-]{20,}\b/g],
  ["github-token", /\b(?:ghp|github_pat)_[A-Za-z0-9_]{20,}\b/g],
  ["aws-access-key", /\bAKIA[0-9A-Z]{16}\b/g],
  ["slack-token", /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/g],
];

const findings = [];
let scanned = 0;

for (const file of files) {
  let stat;
  try {
    stat = statSync(file);
  } catch {
    continue;
  }
  if (!stat.isFile() || stat.size > maxFileSize) continue;

  const buffer = readFileSync(file);
  if (buffer.includes(0)) continue;
  scanned += 1;
  const text = buffer.toString("utf8");

  for (const [name, pattern] of rules) {
    pattern.lastIndex = 0;
    for (const match of text.matchAll(pattern)) {
      const line = text.slice(0, match.index).split("\n").length;
      findings.push(`${file}:${line} (${name})`);
    }
  }
}

if (findings.length > 0) {
  console.error("Potential credential material detected (values intentionally hidden):");
  for (const finding of findings) console.error(`- ${finding}`);
  process.exit(1);
}

console.log(`Secret scan passed: ${scanned} text files checked.`);
