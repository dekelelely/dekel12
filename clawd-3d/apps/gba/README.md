# Clawd for Game Boy Advance

Clawd on a Game Boy Advance, for fun. A 16 MHz GBA with no floating point can't run the 3D engine, so the engine
runs when the ROM is built instead: `build.rs` has `clawd-core` play 25 of his moves (the ones a tap plays in the
apps, from `jump` and `celebrate` to `ultracode`, `dance`, `nap` and `moonwalk`) and `clawd-raster` draw them at
240×160, 15 frames a second, on the apps' dark stage. The ROM (`src/main.rs`) plays the pictures back in the GBA's
8-bit bitmap mode, two pages flipped in the vertical blank. All the frames share one 256-colour palette, and each is
stored as runs of the pixels that differ from him standing still: nearly 800 frames in a 2.5 MB ROM.

## Playing it

Each release has `clawd.gba`. Open it in an emulator such as [mGBA](https://mgba.io), or copy it to a flash cart's SD
card and start it on a GBA, a GBA SP, a Nintendo DS (Slot-2) or a Game Boy Player.

On his own he idles, and every few seconds plays a random move.

| | |
|---|---|
| A | A random move right away |
| L / R | The previous / next move, with its name |
| START | Auto-play: random moves back to back (**AUTO** at the top right) |
| SELECT | Hide or show the hints |

In mGBA's default keys, A is `X`, L and R are `A` and `S`, START is `Enter` and SELECT is `Backspace`.

## Building it yourself

It needs the nightly Rust in `rust-toolchain.toml` (agb rebuilds `core` for the GBA's ARM7, with `rust-src`) and
`agb-gbafix`, which turns the program into a ROM with a valid header:

```sh
rustup toolchain install nightly-2026-07-21 --profile minimal --component rust-src
cargo install agb-gbafix@0.25.0 --locked
cd clawd-3d/apps/gba
cargo build --release
agb-gbafix target/thumbv4t-none-eabi/release/clawd-gba -t CLAWD -o clawd.gba
```

`cargo run --release` builds it and opens it in `mgba-qt`, with agb's log messages on the terminal
(`.cargo/config.toml`). Drawing the frames takes `build.rs` about ten seconds; it draws them again whenever the
engine or `moves.json` changes.

Like `../terminal`, it is its own crate, not a member of `../../rust`'s workspace: it depends on `clawd-core` and
`clawd-raster` only as build dependencies, and on [agb](https://agbrs.dev) for the GBA itself.
