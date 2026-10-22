// Exports Clawd's moves from the JavaScript engine (clawd-engine.js, the one source of the choreography) for the Rust
// port: every frame of every move, resolved the way the engine's targetsFor reads it, plus the idle loop, the camera
// framing and the laptop's screens. Writes rust/clawd-core/moves.json; with --check, fails if that file is out of date.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const enginePath = join(here, "..", "..", "clawd-engine.js");
const outPath = join(here, "..", "clawd-core", "moves.json");

const sandbox = { window: {} };
vm.runInNewContext(readFileSync(enginePath, "utf8"), sandbox, { filename: enginePath });
const E = sandbox.window.ClawdEngine;
const POSES = {
  default: { eyes: "open", arms: "down", feet: "both" },
  "arms-up": { eyes: "open", arms: "up", feet: "both" },
  "look-left": { eyes: "left", arms: "down", feet: "both" },
  "look-right": { eyes: "right", arms: "down", feet: "both" }
};

// One frame as the engine reads it; values left out are 0 / false / the default pose.
function frame(f) {
  const p = typeof f.pose === "string" ? POSES[f.pose] : f.pose.facing ? POSES.default : f.pose;
  if (!p) throw new Error(`unknown pose ${JSON.stringify(f.pose)}`);
  const out = {};
  const put = (key, value, none) => { if (value !== undefined && value !== none) out[key] = value; };
  put("eyes", p.eyes, "open");
  put("arms", p.arms, "down");
  put("feet", p.feet, "both");
  put("offset", f.offset || 0, 0);
  put("x", f.x || 0, 0);
  put("angle", +(f.angle || 0).toFixed(4), 0);
  for (const key of ["lift", "flip", "roll", "squat", "lean", "kick", "strum", "bang", "guitar", "laptop", "dumbbell", "curlL", "curlR"]) {
    put(key, f[key] ? +f[key].toFixed(6) : 0, 0);
  }
  if (f.fret !== undefined) out.fret = f.fret;
  for (const key of ["windmill", "zap", "flash", "typing", "ripple"]) put(key, !!f[key], false);
  put("confetti", f.confetti || 0, 0);
  put("notes", f.notes, undefined);
  put("poof", f.poof, undefined);
  put("shadow", f.shadow, undefined);
  return out;
}

const data = {
  source: "Generated from clawd-engine.js by rust/tools/export-moves.mjs. Don't edit; run the script.",
  frameMs: E.FRAME_MS,
  moves: E.ALL.map((name) => ({ name, frames: E.MOVES[name].map(frame) })),
  idle: E.IDLE.map(frame),
  entrances: E.ENTRANCES,
  onClick: E.ON_CLICK,
  framing: E.FRAMING,
  screens: E.SCREENS,
  laptop: { open: E.LAPTOP_OPEN, typingFrom: E.TYPING_FROM, typingTo: E.TYPING_TO }
};
// One frame per line: readable diffs when a move changes.
const text = JSON.stringify(data, null, 1)
  .replace(/\{\n\s+("eyes"|"arms"|"feet"|"offset"|"x"|"angle"|"lift"|"flip"|"roll"|"squat"|"lean"|"kick"|"strum"|"bang"|"guitar"|"laptop"|"dumbbell"|"curlL"|"curlR"|"fret"|"windmill"|"zap"|"flash"|"typing"|"ripple"|"confetti"|"notes"|"poof"|"shadow")[^{}]*?\n\s*\}/g,
    (m) => m.replace(/\n\s*/g, " ").replace("{ ", "{").replace(" }", "}"))
  .replace(/\{\n\s*\}/g, "{}") + "\n";

if (process.argv.includes("--check")) {
  let current = "";
  try { current = readFileSync(outPath, "utf8"); } catch (err) { /* missing */ }
  if (current !== text) {
    console.error(`${outPath} is out of date: run node rust/tools/export-moves.mjs`);
    process.exit(1);
  }
  console.log("moves.json is up to date");
} else {
  writeFileSync(outPath, text);
  const frames = data.moves.reduce((n, m) => n + m.frames.length, 0);
  console.log(`Wrote ${outPath}: ${data.moves.length} moves, ${frames} frames`);
}
