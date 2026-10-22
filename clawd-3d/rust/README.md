# Clawd in Rust

Clawd's scene without a browser: the same moves as `../clawd-engine.js`, played frame by frame and drawn on the CPU. The Nintendo Switch app runs on it, and `clawd-render` makes pictures and GIFs of any move.

| | |
|---|---|
| `clawd-core` | Plays the moves and poses the model, as the JavaScript engine's `create()` does. `no_std`, so it builds for a Switch. |
| `clawd-raster` | Draws him into RGBA pixels as three.js does on the page: the same lights and highlights, the sun's soft shadow map, the floor that hides him when he sinks into it, the laptop's screen, the guitar's pixel wall, sparks, confetti, dust, notes and the code ripple. |
| `clawd-render` | `clawd-render celebrate --gif --out celebrate.gif`, `clawd-render idle --at 0.5 --out idle.png`, `--bench` to time a move. |
| `clawd-ffi` | The C interface (`clawd-ffi/clawd.h`) the Switch app links. |

## One choreography

The moves live in `../clawd-engine.js` only. `tools/export-moves.mjs` writes every frame of every move to `clawd-core/moves.json`, and the build turns that file into Rust data. After changing a move:

```sh
node clawd-3d/rust/tools/export-moves.mjs     # CI runs it with --check
```

## The same picture

`tools/compare.mjs` draws the same moments with the JavaScript engine (three.js in Chromium) and with this one, from the same random seed, side by side. On average the two differ by under 0.5 of 255 per pixel; the code ripple differs a little more, since it uses an 8×8 pixel font instead of the page's monospace font.

```sh
cargo build --release -p clawd-render
cd tools && npm install playwright && node compare.mjs out celebrate@1.3 ultracode@2.8
```

## Speed

`Clawd::update` says whether the picture changed, so an app draws only then (as the page does), and shadow lookups skip points no caster can reach and settle points wholly in or out of shadow at one read. On one 2.1 GHz core, 640×360 takes 5 ms (idle) to 8 ms (the guitar routine), 1280×720 about four times that.

```sh
cargo test --workspace
cargo run --release -p clawd-render -- ultracode --bench --size 640x360 --ss 1
```
