// Writes latest.yml for the Electron versions of Clawd for Windows (1.22 and older). Their updater reads it from the
// newest release and installs the setup .exe it names, which is now the Rust app's installer.
// Usage: node latest-yml.mjs <Clawd-Setup-1.N.0.exe> <1.N.0>
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { basename } from "node:path";

const [file, version] = process.argv.slice(2);
if (!file || !version) {
  console.error("Usage: node latest-yml.mjs <setup.exe> <version>");
  process.exit(1);
}
const data = readFileSync(file);
const sha512 = createHash("sha512").update(data).digest("base64");
const name = basename(file);
process.stdout.write(
  [
    `version: ${version}`,
    "files:",
    `  - url: ${name}`,
    `    sha512: ${sha512}`,
    `    size: ${data.length}`,
    `path: ${name}`,
    `sha512: ${sha512}`,
    `releaseDate: '${new Date().toISOString()}'`,
    ""
  ].join("\n")
);
