//! The two pictures the engine paints on canvases: the laptop's screen (the session he types) and the wall of pixel
//! cells behind his guitar. Text uses the public-domain 8×8 font (font8x8), which suits his pixel look.
use crate::math::{clamp, js_round};
use alloc::vec;
use alloc::vec::Vec;
use libm::{floor, hypot, round};

/// An RGBA picture, not premultiplied (like a canvas). `version` changes whenever it is redrawn.
pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    /// Sampled nearest (crisp pixel cells) rather than smoothly.
    pub nearest: bool,
    pub version: u32,
}

impl Texture {
    pub fn new(width: usize, height: usize, nearest: bool) -> Texture {
        Texture { width, height, rgba: vec![0; width * height * 4], nearest, version: 0 }
    }

    fn fill(&mut self, x0: usize, y0: usize, w: usize, h: usize, rgb: u32, alpha: f64) {
        let (r, g, b) = (((rgb >> 16) & 255) as f64, ((rgb >> 8) & 255) as f64, (rgb & 255) as f64);
        for y in y0..(y0 + h).min(self.height) {
            for x in x0..(x0 + w).min(self.width) {
                let p = &mut self.rgba[(y * self.width + x) * 4..][..4];
                // source-over, as a canvas composites
                let da = p[3] as f64 / 255.0;
                let oa = alpha + da * (1.0 - alpha);
                if oa <= 0.0 {
                    continue;
                }
                let mix = |s: f64, d: u8| round((s * alpha + d as f64 * da * (1.0 - alpha)) / oa) as u8;
                p[0] = mix(r, p[0]);
                p[1] = mix(g, p[1]);
                p[2] = mix(b, p[2]);
                p[3] = round(oa * 255.0) as u8;
            }
        }
    }

    fn clear(&mut self) {
        self.rgba.fill(0);
    }

    /// One line of 8×8 text with its top-left corner at x, y.
    fn text(&mut self, x: usize, y: usize, text: &str, rgb: u32) {
        for (n, ch) in text.chars().enumerate() {
            let glyph = font8x8::legacy::BASIC_LEGACY.get(ch as usize).copied().unwrap_or([0; 8]);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        self.fill(x + n * 8 + col, y + row, 1, 1, rgb, 1.0);
                    }
                }
            }
        }
    }
}

pub const SCREEN_W: usize = 256;
pub const SCREEN_H: usize = 128;

/// The laptop's screen after `typed` characters of `session`: its last nine lines, coloured like a terminal, and a
/// block cursor after the text.
pub fn draw_screen(t: &mut Texture, session: &str, typed: usize) {
    t.fill(0, 0, t.width, t.height, 0x11181d, 1.0);
    let shown = &session[..typed.min(session.len())];
    let lines: Vec<&str> = shown.split('\n').collect();
    let lines = &lines[lines.len().saturating_sub(9)..];
    for (i, line) in lines.iter().enumerate() {
        let color = if line.starts_with("  +") {
            0x7fe0a0
        } else if line.starts_with('>') {
            0xe8a07f
        } else {
            0x5fd3e4
        };
        t.text(6, 5 + i * 13, line, color);
    }
    let last = lines.last().map_or(0, |l| l.chars().count());
    t.fill(6 + last * 8 + 1, 5 + (lines.len().max(1) - 1) * 13, 6, 8, 0x5fd3e4, 1.0);
    t.version = t.version.wrapping_add(1);
}

pub const WALL_COLS: usize = 18;
pub const WALL_ROWS: usize = 12;
pub const WALL_CELL: usize = 10;

/// The wall behind his guitar: lavender blocks pulse outward from Clawd over deep purple, and flash on a chord.
pub fn draw_wall(t: &mut Texture, clock: f64, flash: f64) {
    t.clear();
    let (cols, rows, cell) = (WALL_COLS as f64, WALL_ROWS as f64, WALL_CELL);
    let (cx, cy) = (cols / 2.0, rows / 2.0 + 1.5); // Clawd's middle, a little below the wall's
    let beat = clock * 2.2;
    let front = (beat - floor(beat)) * 11.0; // a ring of light stepping outward on each beat
    for j in 0..WALL_ROWS {
        for i in 0..WALL_COLS {
            let dx = i as f64 + 0.5 - cx;
            let dy = (j as f64 + 0.5 - cy) * 1.15;
            let d = dx.abs().max(dy.abs()) * 0.6 + hypot(dx, dy) * 0.4;
            let edge = clamp(1.0 - d / 7.5, 0.0, 1.0); // fades out well before the wall ends
            if edge <= 0.0 {
                continue;
            }
            let mut level = if d < front && d > front - 3.5 { 1.0 - (front - d) / 3.5 } else { 0.0 };
            level = level.max(0.45 * clamp(1.0 - d / 4.5, 0.0, 1.0)).max(flash);
            level = js_round(level * edge * 3.0) / 3.0; // stepped, like the app's pixel blocks
            let (x, y) = (i * cell, j * cell);
            t.fill(x, y, cell, cell, 0x3a2496, 0.9 * (edge * 1.6).min(1.0));
            if level > 0.0 {
                t.fill(x + 1, y + 1, cell - 2, cell - 2, 0xbeb6ff, 0.85 * level);
            }
            let line = 0.7 * edge;
            t.fill(x, y, cell, 1, 0x24166e, line);
            t.fill(x, y + cell - 1, cell, 1, 0x24166e, line);
            t.fill(x, y + 1, 1, cell - 2, 0x24166e, line);
            t.fill(x + cell - 1, y + 1, 1, cell - 2, 0x24166e, line);
        }
    }
    t.version = t.version.wrapping_add(1);
}

/// The code ripple's glyphs: the engine's hash, so the same cells show the same characters.
pub fn hash(i: f64, j: f64, k: f64) -> f64 {
    let v = libm::sin(i * 127.1 + j * 311.7 + k * 74.7) * 43758.5453;
    v - floor(v)
}

const GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// The character in cell (i, j) of the code ripple at time t, or None for a gap. Rows read like lines of code: runs of
/// characters broken by spaces, each cell changing now and then.
pub fn glyph_at(i: usize, j: usize, t: f64) -> Option<u8> {
    let (fi, fj) = (i as f64, j as f64);
    if hash(floor(fi / 5.0), fj, 9.0) < 0.18 || hash(fi, fj, 3.0) < 0.12 {
        return None;
    }
    let k = floor((t + hash(fi, fj, 1.0) * 0.6) / 0.35);
    Some(GLYPHS[(floor(hash(fi, fj, k + 17.0) * 997.0) as usize) % GLYPHS.len()])
}

/// An 8×8 glyph's rows (bit 0 is the leftmost pixel).
pub fn glyph_bits(ch: u8) -> [u8; 8] {
    font8x8::legacy::BASIC_LEGACY.get(ch as usize).copied().unwrap_or([0; 8])
}

/// The two shapes of pixel music note (5×5, "#" lit).
pub const NOTE_SHAPES: [[&str; 5]; 2] =
    [["..##.", "..#.#", "..#..", "###..", "###.."], [".####", ".#..#", ".#..#", "##.##", "##.##"]];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_shows_the_typed_text_and_a_cursor() {
        let mut t = Texture::new(SCREEN_W, SCREEN_H, false);
        draw_screen(&mut t, "$ claude\n> hi", 8);
        let lit = t.rgba.chunks(4).filter(|p| p[0] == 0x5f && p[1] == 0xd3).count();
        assert!(lit > 40, "{lit}"); // "$ claude" and the cursor
        assert_eq!(t.version, 1);
    }

    #[test]
    fn wall_is_brightest_in_the_middle() {
        let mut t = Texture::new(WALL_COLS * WALL_CELL, WALL_ROWS * WALL_CELL, true);
        draw_wall(&mut t, 0.0, 0.0);
        let at = |x: usize, y: usize| t.rgba[(y * t.width + x) * 4 + 3];
        assert!(at(95, 75) > 200);
        assert_eq!(at(1, 1), 0); // the corners fade out
    }

    #[test]
    fn glyphs_and_gaps() {
        let cells: Vec<Option<u8>> = (0..200).map(|i| glyph_at(i, 3, 0.4)).collect();
        assert!(cells.iter().any(Option::is_none) && cells.iter().any(Option::is_some));
        assert_eq!(glyph_bits(b'A')[0], 0x0c);
    }
}
