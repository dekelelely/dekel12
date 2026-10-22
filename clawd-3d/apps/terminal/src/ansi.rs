//! Colours for the cells: truecolor when the terminal says it can, the 256-colour palette otherwise.

use std::fmt::Write as _;

/// How to name a colour in an escape sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colors {
    /// `SGR 38;2;r;g;b`, for terminals whose `COLORTERM` says truecolor or 24bit.
    True,
    /// `SGR 38;5;n`, the 256-colour palette.
    Ansi256,
}

impl Colors {
    pub fn detect() -> Colors {
        match std::env::var("COLORTERM") {
            Ok(v) => {
                let v = v.to_ascii_lowercase();
                if v.contains("truecolor") || v.contains("24bit") {
                    Colors::True
                } else {
                    Colors::Ansi256
                }
            }
            Err(_) => Colors::Ansi256,
        }
    }
}

/// Writes cells into a string of escape sequences, skipping a colour change that changes nothing.
pub struct Painter {
    colors: Colors,
    out: String,
    fg: Option<u32>,
    bg: Option<u32>,
}

impl Painter {
    /// Starts a frame at the top left of the screen.
    pub fn new(colors: Colors, capacity: usize) -> Painter {
        let mut out = String::with_capacity(capacity);
        out.push_str("\x1b[H");
        Painter { colors, out, fg: None, bg: None }
    }

    /// One character: a half-block (`▀`), or a bubble's letter.
    pub fn cell(&mut self, ch: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
        self.foreground(fg);
        self.background(bg);
        self.out.push(ch);
    }

    /// The next cell starts the row below.
    pub fn newline(&mut self) {
        self.out.push_str("\r\n");
    }

    fn foreground(&mut self, c: (u8, u8, u8)) {
        let key = pack(c);
        if self.fg == Some(key) {
            return;
        }
        self.fg = Some(key);
        match self.colors {
            Colors::True => {
                write!(self.out, "\x1b[38;2;{};{};{}m", c.0, c.1, c.2).unwrap();
            }
            Colors::Ansi256 => {
                write!(self.out, "\x1b[38;5;{}m", to256(c)).unwrap();
            }
        }
    }

    fn background(&mut self, c: (u8, u8, u8)) {
        let key = pack(c);
        if self.bg == Some(key) {
            return;
        }
        self.bg = Some(key);
        match self.colors {
            Colors::True => {
                write!(self.out, "\x1b[48;2;{};{};{}m", c.0, c.1, c.2).unwrap();
            }
            Colors::Ansi256 => {
                write!(self.out, "\x1b[48;5;{}m", to256(c)).unwrap();
            }
        }
    }

    pub fn finish(mut self) -> String {
        self.out.push_str("\x1b[0m");
        self.out
    }
}

fn pack(c: (u8, u8, u8)) -> u32 {
    (c.0 as u32) << 16 | (c.1 as u32) << 8 | c.2 as u32
}

/// An sRGB colour as the nearest of the 256-colour palette's cube (or grey ramp).
pub fn to256((r, g, b): (u8, u8, u8)) -> u8 {
    if r == g && g == b {
        // The 24-step grey ramp, 8…238, with black and white at its ends.
        if r < 8 {
            16
        } else if r > 248 {
            231
        } else {
            232 + (((r as u16 - 8) * 24 + 123) / 247) as u8
        }
    } else {
        let step = |v: u8| ((v as u16 * 5 + 127) / 255) as u8;
        16 + 36 * step(r) + 6 * step(g) + step(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_is_the_xterm_one() {
        assert_eq!(to256((0, 0, 0)), 16);
        assert_eq!(to256((255, 255, 255)), 231);
        assert_eq!(to256((128, 128, 128)), 244);
        assert_eq!(to256((255, 0, 0)), 196);
        assert_eq!(to256((0, 255, 0)), 46);
        assert_eq!(to256((0, 0, 255)), 21);
    }

    #[test]
    fn a_frame_starts_at_home_and_ends_reset() {
        let mut p = Painter::new(Colors::True, 64);
        p.cell('▀', (1, 2, 3), (1, 2, 3));
        p.cell('▀', (1, 2, 3), (4, 5, 6));
        let s = p.finish();
        assert!(s.starts_with("\x1b[H\x1b[38;2;1;2;3m"));
        assert_eq!(s.matches("\x1b[38;2").count(), 1, "the colour did not change: no second escape");
        assert_eq!(s.matches("\x1b[48;2").count(), 2);
        assert!(s.ends_with("\x1b[0m"));
    }
}
