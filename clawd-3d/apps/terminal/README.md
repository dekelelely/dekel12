# Clawd in the terminal

Clawd in a terminal window, for fun. The same Rust engine as the desktop app and the Switch plays him —
`clawd-core` plays the moves of [`../../clawd-engine.js`](../../clawd-engine.js), `clawd-raster` draws him into
pixels — and this app shows the pixels as half-blocks: two pixels to a cell, `▀` with the top pixel as its
foreground and the bottom one as its background, in truecolor, or in the 256-colour palette when `COLORTERM` isn't
truecolor.

```sh
cd clawd-3d/apps/terminal
cargo run --release
```

It is drawn at the terminal's size, follows a resize, and only when `Clawd::update` says the picture changed, at most
thirty times a second.

| key | |
|---|---|
| `space` | a new random move (like a tap; in the middle of one, it cuts to another) |
| `a` | auto-play on or off |
| `u` | ultra effort: the electric guitar |
| `q`, `Esc`, `Ctrl-C` | quit |

The terminal is put back as it was on the way out, including on a panic.

## One frame, no terminal

`--once <move> --at <seconds>` prints a single frame of that move and exits. It needs no TTY, so it works in CI and
through a pipe; without a terminal it draws 80×24 cells, or `COLUMNS` × `LINES` if they are set.

```sh
cargo run --release -- --once celebrate --at 1.3 > celebrate.ansi
cargo run --release -- --once ultracode --at 2.8 | less -R
```

`--at` walks the move in the same 1/60 s steps as the page and `clawd-render`, so a moment means the same thing
everywhere. The moves are the engine's: `jump`, `look`, `celebrate`, `skip`, `spin`, … `ultracode`, `laptop`,
`moonwalk`; `--help` lists them, and so does a wrong name.

## A speech bubble

`--say "text"` puts a cowsay bubble next to him, and `\n` starts a new line. The text is wrapped to the window and
kept to ASCII, since a letter two cells wide would break the box.

```sh
cargo run --release -- --say "I live in your terminal now"
cargo run --release -- --once celebrate --at 1.3 --say "ship it"
```

## Its own crate

`clawd-term` is not a member of `../../rust`'s workspace: it has its own `Cargo.toml` and `Cargo.lock`, and depends
on `clawd-core` and `clawd-raster` by path. The only other dependency is `crossterm`, for raw mode, the alternate
screen and key events.

```sh
cargo test
cargo build --release
```
