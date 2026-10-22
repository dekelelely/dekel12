//! Clawd on a Game Boy Advance, for fun. The GBA can't run the 3D engine, so build.rs plays and draws every move with
//! it when the ROM is built; this plays the pictures back at 15 frames a second in the 8-bit bitmap mode (mode 4),
//! drawing each one into the page that isn't on screen and flipping pages in the vertical blank. The text is sprites,
//! drawn only when it changes, so it costs the frames nothing.
//!
//! He idles and plays a random move every few seconds on his own. A: a random move. L / R: the previous / next move,
//! with its name. START: auto-play (random moves back to back). SELECT: hide or show the hints.
#![no_std]
#![no_main]

use agb::input::{Button, ButtonController};
use agb::interrupt::VBlank;
use core::ptr::write_volatile;

#[repr(C, align(4))]
struct Aligned<T>(T);

/// A stretch of frames: `idle` loops, every other one is a move that ends where he stands still.
struct Clip {
    name: &'static str,
    first: usize,
    len: usize,
    /// The idle frame to carry on from after the last one.
    resume: usize,
}

mod data {
    use super::{Aligned, Clip};
    include!(concat!(env!("OUT_DIR"), "/data.rs"));
}
use data::{BASE, CLIPS, FONT, FRAMES, OFFSETS, PALETTE};

const W: usize = 240;
const H: usize = 160;
const IDLE: usize = 0;
/// How long he rests between the moves he plays on his own, in frames (15 a second).
const REST: u32 = 4 * 15;
/// The GBA's 60 Hz frames each picture stays up for.
const TICKS: u32 = 4;

const DISPCNT: *mut u16 = 0x0400_0000 as *mut u16;
const BG_PALETTE: *mut u16 = 0x0500_0000 as *mut u16;
const OBJ_PALETTE: *mut u16 = 0x0500_0200 as *mut u16;
const OAM: *mut u16 = 0x0700_0000 as *mut u16;
/// Sprite tile 512: in the bitmap modes the pages take the VRAM below it.
const OBJ_TILES: usize = 0x0601_4000;
const DMA3: *mut u32 = 0x0400_00D4 as *mut u32;
/// Mode 4's two pages: one is shown while the other is drawn.
const PAGES: [usize; 2] = [0x0600_0000, 0x0600_A000];

/// Copies whole words with DMA: the CPU waits, but less than a loop would take.
fn copy_words(to: usize, from: &[u8]) {
    unsafe {
        write_volatile(DMA3, from.as_ptr() as u32);
        write_volatile(DMA3.add(1), to as u32);
        write_volatile(DMA3.add(2), (from.len() / 4) as u32 | 1 << 26 | 1 << 31);
    }
}

/// Pixels, with x and width in fours so whole words can be copied.
#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// A page of mode 4, and where its last frame drew over the stage.
struct Page {
    addr: usize,
    drawn: Rect,
}

impl Page {
    /// Puts the stage (him standing still) back where the last frame went, then draws frame `n` over it.
    fn draw(&mut self, n: usize) {
        let r = self.drawn;
        for y in r.y..r.y + r.h {
            let start = y * W + r.x;
            copy_words(self.addr + start, &BASE.0[start..start + r.w]);
        }
        let data = &FRAMES[OFFSETS[n] as usize..OFFSETS[n + 1] as usize];
        self.drawn =
            Rect { x: data[0] as usize * 4, y: data[1] as usize, w: data[2] as usize * 4, h: data[3] as usize };
        unsafe { runs(data[4..].as_ptr(), self.addr, self.drawn) };
    }
}

/// A frame's runs of pixel pairs (see build.rs's encode); a skip leaves the stage as it is. This is the work of every
/// frame, so it runs from IWRAM as ARM code, the fastest the GBA has.
#[unsafe(link_section = ".iwram")]
#[instruction_set(arm::a32)]
unsafe fn runs(mut p: *const u8, page: usize, r: Rect) {
    unsafe {
        for y in r.y..r.y + r.h {
            let mut to = (page + y * W + r.x) as *mut u16;
            let end = to.add(r.w / 2);
            while to < end {
                let op = *p as usize;
                p = p.add(1);
                if op < 0x40 {
                    to = to.add(op + 1);
                } else if op < 0x80 {
                    let pair = *p as u16 | (*p.add(1) as u16) << 8;
                    p = p.add(2);
                    for _ in 0..op - 0x40 + 1 {
                        write_volatile(to, pair);
                        to = to.add(1);
                    }
                } else {
                    for _ in 0..op - 0x80 + 1 {
                        write_volatile(to, *p as u16 | (*p.add(1) as u16) << 8);
                        p = p.add(2);
                        to = to.add(1);
                    }
                }
            }
        }
    }
}

// The text's sprite palette.
const NAME: u8 = 1;
const SHADOW: u8 = 2;
const HINT: u8 = 3;
const CLAY: u8 = 4;
const TEXT_COLOURS: [u16; 5] = [0, rgb15(0xe6e8eb), rgb15(0x000000), rgb15(0x8b949e), rgb15(0xd77757)];

const fn rgb15(c: u32) -> u16 {
    let r = (c >> 16 & 255) * 31 / 255;
    let g = (c >> 8 & 255) * 31 / 255;
    let b = (c & 255) * 31 / 255;
    (r | g << 5 | b << 10) as u16
}

/// A line of text is eight 32×16 sprites side by side: 256×16 pixels, 64 tiles at 4 bits a pixel.
const SPRITES: usize = 8;

/// Draws `s` (the engine's 8×8 font8x8, with a shadow) into line `slot`'s tiles and puts its sprites at x, y.
fn show_line(slot: usize, x: usize, y: usize, s: &str, colour: u8) {
    let mut tiles = [0u32; SPRITES * 8 * 8];
    let mut dot = |px: usize, py: usize, c: u8| {
        // A sprite's tiles go row by row, four to a row; a tile's row of eight pixels is a word.
        let tile = px / 32 * 8 + py / 8 * 4 + px % 32 / 8;
        let (word, shift) = (tile * 8 + py % 8, px % 8 * 4);
        tiles[word] = tiles[word] & !(15 << shift) | (c as u32) << shift;
    };
    let s = &s[..s.len().min(31)];
    for (pass, dx, colour) in [(0, 1, SHADOW), (1, 0, colour)] {
        for (i, ch) in s.bytes().enumerate() {
            let glyph = FONT[ch.clamp(b' ', b'~') as usize - b' ' as usize];
            for (gy, bits) in glyph.iter().enumerate() {
                for gx in (0..8).filter(|gx| bits & 1 << gx != 0) {
                    dot(i * 8 + gx + dx, gy + 1 - pass, colour);
                }
            }
        }
    }
    let at = OBJ_TILES + slot * SPRITES * 8 * 32;
    for (i, word) in tiles.iter().enumerate() {
        unsafe { write_volatile((at as *mut u32).add(i), *word) };
    }
    let used = (s.len() * 8 + 1).div_ceil(32);
    for i in 0..SPRITES {
        let sprite = slot * SPRITES + i;
        unsafe {
            let attrs = OAM.add(sprite * 4);
            if i < used {
                // Wide and 4 bits a pixel; 32×16; its tiles.
                write_volatile(attrs, y as u16 | 1 << 14);
                write_volatile(attrs.add(1), ((x + i * 32) & 511) as u16 | 2 << 14);
                write_volatile(attrs.add(2), (512 + sprite * 8) as u16);
            } else {
                write_volatile(attrs, 1 << 9);
            }
        }
    }
}

fn hide_line(slot: usize) {
    for sprite in slot * SPRITES..(slot + 1) * SPRITES {
        unsafe { write_volatile(OAM.add(sprite * 4), 1 << 9) };
    }
}

/// What the text says, to redraw only what changes.
#[derive(Clone, Copy, PartialEq)]
struct Text {
    name: Option<usize>,
    auto: bool,
    hints: bool,
}

impl Text {
    fn show(self, was: Option<Text>) {
        if was.map(|t| t.name) != Some(self.name) {
            match self.name {
                Some(clip) => show_line(0, 8, 8, CLIPS[clip].name, NAME),
                None => hide_line(0),
            }
        }
        if was.map(|t| t.auto) != Some(self.auto) {
            if self.auto {
                show_line(1, W - 8 - 4 * 8, 8, "AUTO", CLAY);
            } else {
                hide_line(1);
            }
        }
        if was.map(|t| t.hints) != Some(self.hints) {
            if self.hints {
                show_line(2, 8, 136, "A: random move  L/R: pick", HINT);
                show_line(3, 8, 147, "START: auto  SELECT: hide", HINT);
            } else {
                hide_line(2);
                hide_line(3);
            }
        }
    }
}

/// What plays: the clip and frame to show next, and the switches.
struct Player {
    clip: usize,
    frame: usize,
    /// The move L and R step from: the last one played.
    picked: usize,
    rest: u32,
    auto: bool,
    hints: bool,
}

impl Player {
    fn play(&mut self, clip: usize) {
        self.clip = clip;
        self.frame = 0;
        self.picked = clip;
    }

    /// Any move but the last one.
    fn random(&self) -> usize {
        let moves = CLIPS.len() - 1;
        loop {
            let m = 1 + agb::rng::next_i32().unsigned_abs() as usize % moves;
            if m != self.picked || moves == 1 {
                return m;
            }
        }
    }

    /// Buttons; true if the picture should change right away.
    fn buttons(&mut self, input: &ButtonController) -> bool {
        let moves = CLIPS.len() - 1;
        if input.is_just_pressed(Button::A) {
            self.play(self.random());
        } else if input.is_just_pressed(Button::L) {
            self.play(if self.picked <= 1 { moves } else { self.picked - 1 });
        } else if input.is_just_pressed(Button::R) {
            self.play(if self.picked >= moves { 1 } else { self.picked + 1 });
        } else if input.is_just_pressed(Button::Start) {
            self.auto = !self.auto;
            if self.auto && self.clip == IDLE {
                self.play(self.random());
            }
        } else if input.is_just_pressed(Button::Select) {
            self.hints = !self.hints;
        } else {
            return false;
        }
        true
    }

    /// On to the next frame: idle loops, a move ends back in idle, and he picks his own moves.
    fn advance(&mut self) {
        let clip = &CLIPS[self.clip];
        self.frame += 1;
        if self.clip == IDLE {
            self.frame %= clip.len;
            self.rest += 1;
            if self.auto || self.rest >= REST {
                self.play(self.random());
            }
        } else if self.frame == clip.len {
            if self.auto {
                self.play(self.random());
            } else {
                self.clip = IDLE;
                self.frame = clip.resume;
                self.rest = 0;
            }
        }
    }

    fn frame(&self) -> usize {
        CLIPS[self.clip].first + self.frame
    }

    fn text(&self) -> Text {
        Text { name: (self.clip != IDLE).then_some(self.clip), auto: self.auto, hints: self.hints }
    }
}

#[agb::entry]
fn main(_gba: agb::Gba) -> ! {
    for (i, c) in PALETTE.iter().enumerate() {
        unsafe { write_volatile(BG_PALETTE.add(i), *c) };
    }
    for (i, c) in TEXT_COLOURS.iter().enumerate() {
        unsafe { write_volatile(OBJ_PALETTE.add(i), *c) };
    }
    for slot in 0..128 / SPRITES {
        hide_line(slot);
    }
    // The first frame puts the whole stage down.
    let mut pages = PAGES.map(|addr| Page { addr, drawn: Rect { x: 0, y: 0, w: W, h: H } });
    let vblank = VBlank::get();
    let mut input = ButtonController::new();
    // His first move comes a couple of seconds in.
    let mut player = Player { clip: IDLE, frame: 0, picked: 0, rest: REST - 30, auto: false, hints: true };
    let (mut shown, mut text) = (0, None);
    loop {
        let back = 1 - shown;
        pages[back].draw(player.frame());
        let mut ticks = 0;
        while ticks < TICKS {
            vblank.wait_for_vblank();
            ticks += 1;
            input.update();
            let _ = agb::rng::next_i32(); // so when a button is pressed decides what comes next
            if player.buttons(&input) {
                pages[back].draw(player.frame());
                vblank.wait_for_vblank();
                break;
            }
        }
        // Mode 4, the page just drawn, 1D sprite tiles, background 2 and sprites: switched in the vertical blank.
        unsafe { write_volatile(DISPCNT, 4 | (back as u16) << 4 | 1 << 6 | 1 << 10 | 1 << 12) };
        let now = player.text();
        if text != Some(now) {
            now.show(text);
            text = Some(now);
        }
        shown = back;
        player.advance();
    }
}
