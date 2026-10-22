//! `--say "text"`: a cowsay speech bubble beside him, with a tail pointing his way.

/// A letter of the bubble, over the picture. The cell's background hides whatever it covers.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
}

const PAPER: (u8, u8, u8) = (0x12, 0x16, 0x1b);
const INK: (u8, u8, u8) = (0xf2, 0xf6, 0xff);
const EDGE: (u8, u8, u8) = (0x7a, 0xd7, 0xe6);

/// The wrapped text, in the terminal cells it will be drawn in.
pub struct Bubble {
    lines: Vec<String>,
    /// The text's width; the box is four cells wider (`< `, ` >`).
    inner: usize,
}

impl Bubble {
    /// Wraps `text` to fit a terminal of `cols` × `rows` cells; `None` when there is no room for a bubble.
    pub fn new(text: &str, cols: usize, rows: usize) -> Option<Bubble> {
        if cols < 16 || rows < 6 {
            return None;
        }
        let max_inner = (cols - 6).min(46);
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            lines.extend(wrap(paragraph, max_inner));
        }
        let inner = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        if inner == 0 {
            return None;
        }
        Some(Bubble { lines, inner })
    }

    /// The bubble's cells, as many as the screen has, so they can be laid over a frame.
    pub fn cells(&self, cols: usize, rows: usize) -> Vec<Option<Cell>> {
        let mut out = vec![None; cols * rows];
        let width = self.inner + 4;
        let height = self.lines.len() + 2;
        // Up in the corner beside him, a row clear of the edge, and clear of the top unless the screen is short.
        let top = if rows >= height + 3 { 1 } else { 0 };
        let left = cols.saturating_sub(width + 1);
        let mut put = |row: usize, col: usize, ch: char, fg: (u8, u8, u8)| {
            if row < rows && col < cols {
                out[row * cols + col] = Some(Cell { ch, fg, bg: PAPER });
            }
        };
        //  ____
        // The first and last cells of the row are blank, as cowsay has them.
        put(top, left, ' ', EDGE);
        for i in 0..width - 2 {
            put(top, left + 1 + i, '_', EDGE);
        }
        put(top, left + width - 1, ' ', EDGE);
        for (n, line) in self.lines.iter().enumerate() {
            let row = top + 1 + n;
            let (open, close) = if n == 0 { ('<', '>') } else { ('|', '|') };
            put(row, left, open, EDGE);
            put(row, left + 1, ' ', INK);
            let mut col = left + 2;
            for ch in line.chars() {
                put(row, col, ch, INK);
                col += 1;
            }
            for c in col..left + width - 1 {
                put(row, c, ' ', INK);
            }
            put(row, left + width - 1, close, EDGE);
        }
        //  ----
        let bottom = top + height - 1;
        put(bottom, left, ' ', EDGE);
        for i in 0..width - 2 {
            put(bottom, left + 1 + i, '-', EDGE);
        }
        put(bottom, left + width - 1, ' ', EDGE);
        // The tail, pointing down and left, his way.
        put(bottom + 1, left + 2, '\\', EDGE);
        put(bottom + 2, left + 1, '\\', EDGE);
        out
    }
}

/// ASCII only: a letter that is two cells wide would break the box.
fn ascii(word: &str) -> String {
    word.chars().map(|c| if c.is_ascii_graphic() { c } else { '?' }).collect()
}

/// Greedy word wrap, splitting a word too long for one line.
fn wrap(paragraph: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        let mut word = ascii(word);
        while word.chars().count() > max {
            let head: String = word.chars().take(max).collect();
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            lines.push(head);
            word = word.chars().skip(max).collect();
        }
        if word.is_empty() {
            continue;
        }
        let len = line.chars().count();
        if len == 0 {
            line = word;
        } else if len + 1 + word.chars().count() <= max {
            line.push(' ');
            line.push_str(&word);
        } else {
            lines.push(std::mem::take(&mut line));
            line = word;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(b: &Bubble, cols: usize, rows: usize) -> Vec<String> {
        let cells = b.cells(cols, rows);
        (0..rows)
            .map(|r| {
                (0..cols)
                    .map(|c| cells[r * cols + c].map(|cell| cell.ch).unwrap_or(' '))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn it_is_a_box_with_his_text_in_it() {
        let b = Bubble::new("hello, Clawd", 60, 20).unwrap();
        let art = drawn(&b, 60, 20);
        assert_eq!(art[1].trim(), "______________");
        assert_eq!(art[2].trim(), "< hello, Clawd >");
        assert_eq!(art[3].trim(), "--------------");
        assert_eq!(art[4].trim(), "\\");
    }

    #[test]
    fn long_text_wraps_and_stays_inside_the_terminal() {
        let b =
            Bubble::new("a message that is quite a lot longer than the bubble can hold on one line", 40, 20).unwrap();
        let art = drawn(&b, 40, 20);
        assert!(art.iter().all(|l| l.chars().count() <= 40));
        assert!(b.lines.len() > 1);
        assert!(b.lines.iter().all(|l| l.chars().count() <= 34));
        // Every line is inside the box.
        for line in &art[2..2 + b.lines.len()] {
            let line = line.trim();
            assert!(line.starts_with('<') || line.starts_with('|'), "{line:?}");
            assert!(line.ends_with('>') || line.ends_with('|'), "{line:?}");
        }
    }

    #[test]
    fn a_word_too_long_for_a_line_is_broken() {
        let b = Bubble::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", 30, 20).unwrap();
        assert!(b.lines.len() > 1);
        assert!(b.lines.iter().all(|l| l.chars().count() <= 24));
    }

    #[test]
    fn a_tiny_terminal_gets_no_bubble() {
        assert!(Bubble::new("hi", 10, 2).is_none());
        assert!(Bubble::new("   ", 60, 20).is_none());
    }
}
