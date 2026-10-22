//! Clawd in the terminal: his moves, played by the Rust engine (clawd-core) and drawn by clawd-raster as half-block
//! characters, two pixels to a cell, in truecolor or the 256-colour palette.
//!
//!   clawd-term                      watch him, and press keys
//!   clawd-term --say "ship it"      with a speech bubble
//!   clawd-term --once celebrate --at 1.3    one frame, no terminal needed

use clawd_core::{moves, Clawd, Mode, Options, Rng};
use clawd_raster::{Background, Renderer, Settings};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};
use std::error::Error;
use std::io::{self, IsTerminal, Write};
use std::panic;
use std::process::exit;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod ansi;
mod bubble;

use ansi::{Colors, Painter};
use bubble::{Bubble, Cell};

/// The fastest he is drawn: the engine says which frames changed, and no more than thirty a second are shown.
const FRAME: Duration = Duration::from_nanos(33_333_334);
const STEP: f64 = 1.0 / 60.0;

const USAGE: &str = "\
usage: clawd-term [--say TEXT]
       clawd-term --once <move> --at <seconds> [--say TEXT]

Watch Clawd, drawn with half-blocks in a terminal:

  space        a new random move (like a tap)
  a            auto-play on or off
  u            ultra effort (the electric guitar)
  q            quit (Esc and Ctrl-C too)

options:
  --once <move>   print one frame of that move and exit; no terminal needed
  --at <seconds>  how far into the move that frame is (default 0)
  --say <text>    put a speech bubble next to him; \\n starts a new line
  -h, --help      this";

struct Args {
    once: Option<String>,
    at: f64,
    say: Option<String>,
}

fn main() {
    let args = match parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("clawd-term: {e}\n\n{USAGE}");
            exit(2);
        }
    };
    match run(&args) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("clawd-term: {e}");
            exit(1);
        }
    }
}

fn parse() -> Result<Args, String> {
    let mut args = Args { once: None, at: 0.0, say: None };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| it.next().ok_or(format!("{flag} needs a value"));
        match arg.as_str() {
            "--once" => args.once = Some(value("--once")?),
            "--at" => {
                let v = value("--at")?;
                args.at = v.parse().map_err(|_| format!("--at: not a number of seconds: {v}"))?;
                if args.at.is_nan() || args.at < 0.0 {
                    return Err("--at: seconds cannot be negative".into());
                }
            }
            "--say" => args.say = Some(value("--say")?),
            "-h" | "--help" => {
                println!("{USAGE}\n\nmoves: {}", move_names().join(", "));
                exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    Ok(args)
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    match &args.once {
        Some(name) => once(args, name),
        None => interactive(args),
    }
}

fn move_names() -> Vec<&'static str> {
    moves::MOVES.iter().map(|m| m.name).collect()
}

/// A frame of one move, printed and gone: `--once celebrate --at 1.3`. No terminal is opened, so a pipe or CI is fine.
fn once(args: &Args, name: &str) -> Result<(), Box<dyn Error>> {
    let (cols, rows) = size();
    let mut screen = Screen::new(cols, rows, args.say.as_deref(), 1);
    if !screen.clawd.play_from(name, true, 0) {
        return Err(format!("no move called {name:?}; try one of: {}", move_names().join(", ")).into());
    }
    screen.clawd.update(0.0);
    // The same step the page and clawd-render walk, so `--at` means the same moment in both.
    let mut t = 0.0;
    while t + STEP / 2.0 < args.at {
        screen.clawd.update(STEP);
        t += STEP;
    }
    let frame = screen.draw();
    let mut out = io::stdout().lock();
    out.write_all(frame.as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// His size in the terminal, or a sensible one when there is no terminal to ask.
fn size() -> (usize, usize) {
    let (mut cols, mut rows) = if io::stdout().is_terminal() {
        terminal::size().unwrap_or((80, 24))
    } else {
        let from_env = |key: &str, fallback: u16| {
            std::env::var(key).ok().and_then(|v| v.parse().ok()).filter(|v: &u16| *v >= 4).unwrap_or(fallback)
        };
        (from_env("COLUMNS", 80), from_env("LINES", 24))
    };
    cols = cols.max(16);
    rows = rows.max(6);
    (cols as usize, rows as usize)
}

fn interactive(args: &Args) -> Result<(), Box<dyn Error>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("not a terminal: use --once <move> --at <seconds> to print a frame instead".into());
    }
    let _term = Terminal::enter()?;
    let (cols, rows) = size();
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u32).unwrap_or(1);
    let mut screen = Screen::new(cols, rows, args.say.as_deref(), seed);
    let mut rng = Rng::new(seed ^ 0x5eed);

    let mut out = io::stdout();
    out.write_all(screen.draw().as_bytes())?;
    out.flush()?;

    let mut tick = Instant::now();
    let mut redraw = false;
    loop {
        let wait = FRAME.saturating_sub(tick.elapsed());
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                    if quit(&key) {
                        break;
                    }
                    match key.code {
                        KeyCode::Char(' ') => next_move(&mut screen.clawd, &mut rng),
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            screen.clawd.set_auto(!screen.clawd.auto());
                        }
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            screen.clawd.set_ultra(!screen.clawd.ultra());
                        }
                        _ => {}
                    }
                }
                Event::Resize(cols, rows) => {
                    screen.resize(cols as usize, rows as usize);
                    redraw = true;
                }
                _ => {}
            }
        }
        // The frame is due at most thirty times a second; keys and resizes are answered as they come.
        let elapsed = tick.elapsed();
        if elapsed < FRAME {
            continue;
        }
        tick = Instant::now();
        if screen.clawd.update(elapsed.as_secs_f64().min(0.1)) || redraw {
            redraw = false;
            out.write_all(screen.draw().as_bytes())?;
            out.flush()?;
        }
    }
    Ok(())
}

fn quit(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => true,
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

/// A tap's new random move: while he idles the engine's tap does it; in the middle of a move, cut to another one.
fn next_move(clawd: &mut Clawd, rng: &mut Rng) {
    if clawd.current() == "idle" {
        clawd.tap();
        return;
    }
    let now = clawd.current();
    let others: Vec<&'static str> = moves::ON_CLICK.iter().copied().filter(|n| *n != now).collect();
    let pick = (rng.next_f64() * others.len() as f64) as usize;
    clawd.play(others[pick.min(others.len() - 1)]);
}

/// The picture and the engine that poses it, at the terminal's size: one pixel a column, two a row.
struct Screen {
    clawd: Clawd,
    renderer: Renderer,
    cols: usize,
    rows: usize,
    say: Option<String>,
    colors: Colors,
}

impl Screen {
    fn new(cols: usize, rows: usize, say: Option<&str>, seed: u32) -> Screen {
        let (cols, rows) = (cols.max(16), rows.max(6));
        let mut clawd = Clawd::new(Options { mode: Mode::Stage, width: cols as f64, height: (rows * 2) as f64, seed });
        // The apps' dark stage.
        clawd.set_colors(0x2b3138, 0x4a525c, 0x4a525c, 0x4cc9db);
        clawd.update(0.0); // the engine's first frame, as on the page
        let colors = Colors::detect();
        Screen {
            clawd,
            renderer: Renderer::new(cols, rows * 2, settings()),
            cols,
            rows,
            say: say.map(str::to_string),
            colors,
        }
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        let (cols, rows) = (cols.max(16), rows.max(6));
        if (cols, rows) == (self.cols, self.rows) {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.clawd.resize(cols as f64, (rows * 2) as f64);
        self.renderer.resize(cols, rows * 2);
    }

    /// One screenful of escape sequences: `▀` with the top pixel as its foreground and the bottom one as its
    /// background, and the speech bubble's letters over the top.
    fn draw(&mut self) -> String {
        let (cols, rows) = (self.cols, self.rows);
        let bubble = self.say.as_deref().and_then(|t| Bubble::new(t, cols, rows)).map(|b| b.cells(cols, rows));
        let pixels = self.renderer.render(&self.clawd);
        let mut p = Painter::new(self.colors, cols * rows * 12);
        let pixel = |x: usize, y: usize| {
            let i = (y * cols + x) * 4;
            (pixels[i], pixels[i + 1], pixels[i + 2])
        };
        for row in 0..rows {
            for col in 0..cols {
                if let Some(Cell { ch, fg, bg }) = bubble.as_ref().and_then(|b| b[row * cols + col]) {
                    p.cell(ch, fg, bg);
                } else {
                    p.cell('▀', pixel(col, row * 2), pixel(col, row * 2 + 1));
                }
            }
            if row + 1 < rows {
                p.newline();
            }
        }
        p.finish()
    }
}

fn settings() -> Settings {
    Settings {
        background: Background::App,
        // A terminal picture is small: a 512-texel shadow map is plenty, and 2048 would cost more than the pixels do.
        shadow_size: 512,
        soft_shadows: true,
        // Rendered at twice the size and averaged down: the cells are big, so the edges need it.
        supersample: 2,
        effects: true,
    }
}

/// Raw mode, the alternate screen and no cursor while he is up; the same undone by `Drop`, and again by the panic
/// hook, so a panic leaves the shell as it was.
struct Terminal;

impl Terminal {
    fn enter() -> io::Result<Terminal> {
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        if let Err(e) = execute!(out, terminal::EnterAlternateScreen, cursor::Hide) {
            let _ = terminal::disable_raw_mode();
            return Err(e);
        }
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        Ok(Terminal)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let mut out = io::stdout();
    let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_the_size_of_the_terminal() {
        let mut screen = Screen::new(40, 12, None, 5);
        let frame = screen.draw();
        // One `▀` a cell, and a carriage return between rows (none after the last).
        let cells = frame.matches('▀').count();
        assert_eq!(cells, 40 * 12);
        assert_eq!(frame.matches("\r\n").count(), 11);
    }

    #[test]
    fn the_bubble_replaces_cells_without_moving_the_picture() {
        let plain = Screen::new(60, 20, None, 5).draw();
        let said = Screen::new(60, 20, Some("hello, Clawd"), 5).draw();
        assert!(said.contains("Clawd"));
        assert!(said.matches('▀').count() < plain.matches('▀').count());
    }

    #[test]
    fn every_move_plays() {
        let mut clawd = Clawd::new(Options { mode: Mode::Stage, width: 40.0, height: 24.0, seed: 3 });
        for name in move_names() {
            assert!(clawd.play(name), "{name}");
        }
    }
}
