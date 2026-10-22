// Draws the same moments with the JavaScript engine (three.js, in Chromium) and the Rust engine (clawd-render), side
// by side, and measures how far apart they are. Both run the same frames from the same seed.
// Usage: node compare.mjs <out dir> [move@seconds ...]   (needs Playwright, three.js from clawd-3d/apps/node_modules
// and a release build of clawd-render)
import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const [out = "compare", ...asked] = process.argv.slice(2);
const moments = asked.length ? asked : ["idle@0.5", "jump@0.25", "celebrate@1.3", "back-flip@0.6", "laptop@1.5", "dumbbell-curl@1.0", "ultracode@2.8", "peekaboo@0.4"];
const [W, H, SEED] = [640, 400, 10];
mkdirSync(out, { recursive: true });

// Math.random seeded as the Rust engine's generator, and time ours: frames run when told, 1/60 s apart.
const stub = `<script>
let s = (${SEED} % 2147483646) + 1;
Math.random = () => ((s = (s * 16807) % 2147483647) / 2147483647);
// three.js takes random numbers too, for the ids of what it builds on the first frame: start counting after that.
window.__reseed = () => { s = (${SEED} % 2147483646) + 1; };
let queue = [];
window.requestAnimationFrame = (fn) => queue.push(fn);
window.__step = (now) => { const q = queue; queue = []; q.forEach((fn) => fn(now)); };
</script>`;
const page = `<!doctype html><html style="height:100%;background:radial-gradient(120% 90% at 50% 30%, #1e2227, #0d0f12)"><head>${stub}</head><body style="margin:0">
<div id="stage" style="position:fixed;inset:0"><canvas id="c" style="position:absolute;inset:0;width:100%;height:100%"></canvas>
<canvas id="f" style="position:absolute;inset:0;width:100%;height:100%"></canvas></div>
<script>${readFileSync(join(root, "apps", "node_modules", "three", "build", "three.min.js"), "utf8")}</script>
<script>${readFileSync(join(root, "clawd-engine.js"), "utf8")}</script></body></html>`;

const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const results = [];
for (const m of moments) {
  const [name, at] = m.split("@");
  const tab = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
  await tab.setContent(page);
  // The first frame (three.js builds what it needs), then a pause for the page's resize observer, which clears the
  // overlay canvas when it first runs.
  await tab.evaluate(() => {
    window.__c = window.ClawdEngine.create({ stage: document.getElementById("stage"), canvas: document.getElementById("c"), fx: document.getElementById("f"), mode: "stage" });
    window.__c.setColors({ floor: "#2b3138", dots: "#4a525c", dust: "#4a525c", code: "#4cc9db" });
    window.__now = 1000;
    window.__step(window.__now);
  });
  await tab.waitForTimeout(150);
  await tab.evaluate(({ name, at }) => {
    window.__reseed();
    window.__c.play(name, { snap: true });
    for (let t = 0; t + 1 / 120 < +at; t += 1 / 60) window.__step((window.__now += 1000 / 60));
  }, { name, at });
  const js = join(out, `${name}-js.png`), rs = join(out, `${name}-rust.png`);
  await tab.waitForTimeout(150);   // the overlay canvas reaches the screen on the browser's next frame
  await tab.screenshot({ path: js });
  await tab.close();
  execFileSync(join(here, "..", "target", "release", "clawd-render"), [name, "--at", at, "--size", `${W}x${H}`, "--scale", "2", "--ss", "2", "--seed", String(SEED), "--out", rs]);
  // Compare and put side by side (Python + Pillow).
  const stats = execFileSync("python3", ["-c", `
import sys
from PIL import Image, ImageChops, ImageStat
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB").resize(a.size)
d = ImageChops.difference(a, b).convert("L")
hist = d.histogram(); n = sum(hist)
close = sum(hist[:25]) / n
print(f"{ImageStat.Stat(d).mean[0]:.2f} {close*100:.1f}")
im = Image.new("RGB", (a.width * 2 + 8, a.height), "white"); im.paste(a, (0, 0)); im.paste(b, (a.width + 8, 0)); im.save(sys.argv[3])
`, js, rs, join(out, `${name}-side-by-side.png`)]).toString().trim().split(" ");
  results.push({ moment: m, meanDifference: +stats[0], pixelsWithin10Percent: +stats[1] });
  console.log(`${m}: mean difference ${stats[0]} of 255, ${stats[1]}% of pixels within 25 of 255`);
}
writeFileSync(join(out, "results.json"), JSON.stringify(results, null, 2));
await browser.close();
