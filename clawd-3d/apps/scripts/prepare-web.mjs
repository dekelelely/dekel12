// Copies the page, the shared engine and three.js into each app's web folder.
// Run `npm install` in clawd-3d/apps first; then `npm run prepare-web`.
import { cpSync, mkdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const apps = join(dirname(fileURLToPath(import.meta.url)), "..");
const files = {
  "index.html": join(apps, "web", "index.html"),
  "clawd-engine.js": join(apps, "..", "clawd-engine.js"),
  "three.min.js": join(apps, "node_modules", "three", "build", "three.min.js")
};
const targets = [
  join(apps, "desktop", "web"),
  join(apps, "android", "app", "src", "main", "assets", "www")
];

for (const [name, from] of Object.entries(files)) {
  if (!existsSync(from)) {
    console.error(`Missing ${from}. Run "npm install" in clawd-3d/apps first.`);
    process.exit(1);
  }
}
for (const dir of targets) {
  mkdirSync(dir, { recursive: true });
  for (const [name, from] of Object.entries(files)) cpSync(from, join(dir, name));
  console.log(`Copied web files to ${dir}`);
}
