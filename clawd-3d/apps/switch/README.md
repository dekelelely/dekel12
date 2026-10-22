# Clawd for Nintendo Switch

Clawd on a Switch running homebrew: the Rust engine (`clawd-3d/rust`) plays his moves and draws him at 640×360 on the CPU, and `source/main.c` shows him twice the size on the 1280×720 screen. Like floating Clawd on the desktop, he plays a move every few seconds on his own, and every tap on him plays a new one. Docked, the Switch scales the picture up to the TV.

## Getting it

Every build of the apps on GitHub also builds `clawd.nro` (see the repository's Releases page). Copy it to `/switch/` on the SD card and open it from the Homebrew Menu. Open the Homebrew Menu through a game (hold R while starting any title) rather than the Album, if you can: an applet gets less memory and CPU time, though Clawd runs either way.

## Controls

| | |
|---|---|
| Tap Clawd | A new move right away |
| Drag on the screen, or the right stick | Turn the stage |
| Left stick, or a finger on the screen | Where he looks |
| A | A random move |
| L / R | The previous / next move, in order |
| X | Auto-play: random moves back to back |
| Y | Ultra effort: the electric guitar, kept between moves |
| B | Working: he types on his laptop until you press B again |
| − | Hide or show the hints |
| + | Quit |

## Building it yourself

You need [devkitPro](https://devkitpro.org/wiki/Getting_Started) with devkitA64 and libnx (`switch-dev`), and [rustup](https://rustup.rs). The Makefile builds the Rust engine first with the nightly toolchain it names (`RUST_TOOLCHAIN`), because core and alloc have to be rebuilt position-independent for an NRO (`-Z build-std`):

```sh
rustup toolchain install nightly-2026-10-15 --profile minimal --component rust-src
cd clawd-3d/apps/switch
make                        # clawd.nro
make APP_VERSION=1.23.0     # with a version for the Homebrew Menu
```

Or with Docker, no devkitPro install needed (this is what CI runs):

```sh
docker run --rm -v "$PWD":/src -w /src/clawd-3d/apps/switch devkitpro/devkita64 ./ci-build.sh 1.23.0
```

`make clean` removes the build, the Rust engine's Switch build included.
