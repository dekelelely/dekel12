// Render SVGs to PNG with the pre-installed Chromium.
// Usage: node render.mjs <size> <out.png> <in.svg> [<out.png> <in.svg> ...]
import { createRequire } from "module";
import { readFileSync } from "fs";
const require = createRequire("/opt/node-tools/node_modules/");
const { chromium } = require("playwright");

const [size, ...pairs] = process.argv.slice(2);
const px = Number(size);
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: px, height: px } });
for (let i = 0; i < pairs.length; i += 2) {
  const [out, src] = [pairs[i], pairs[i + 1]];
  const svg = readFileSync(src, "utf8");
  const m = svg.match(/viewBox="([\d.\s-]+)"/);
  const [, , w, h] = m[1].trim().split(/\s+/).map(Number);
  const H = Math.round((px * h) / w);
  await page.setViewportSize({ width: px, height: H });
  await page.setContent(
    `<html><body style="margin:0;background:transparent">` +
      svg.replace("<svg ", `<svg width="${px}" height="${H}" `) +
      `</body></html>`
  );
  await page.screenshot({ path: out, omitBackground: true });
}
await browser.close();
