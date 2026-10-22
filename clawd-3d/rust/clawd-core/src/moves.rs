//! The moves, frame by frame, as clawd-engine.js defines them (exported to moves.json; build.rs turns that into the
//! statics below). A frame lasts FRAME_MS; Clawd eases toward each frame's pose.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eyes {
    Open,
    Left,
    Right,
    Closed,
    Wink,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arms {
    Down,
    Up,
    OneUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feet {
    Both,
    Left,
    Right,
}

/// Pixel music notes for this frame (the guitar routine): over his head, or beside him.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notes {
    None,
    Above,
    Side,
}

/// A puff of dust at his feet: "·" small, "~" wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poof {
    None,
    Dot,
    Wave,
}

/// The sprite's shadow under a jump: his height above the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shadow {
    None,
    Wide,
    Narrow,
}

/// One frame, resolved the way the engine's targetsFor reads it. Angles are in degrees; `x` is in terminal columns.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub eyes: Eyes,
    pub arms: Arms,
    pub feet: Feet,
    pub offset: i8,
    pub x: f64,
    pub angle: f64,
    pub lift: f64,
    pub flip: f64,
    pub roll: f64,
    pub squat: f64,
    pub lean: f64,
    pub kick: f64,
    pub strum: f64,
    pub bang: f64,
    pub guitar: f64,
    pub laptop: f64,
    pub dumbbell: f64,
    pub curl_l: f64,
    pub curl_r: f64,
    /// Where the fretting hand is on the neck; NaN keeps the last one.
    pub fret: f64,
    pub windmill: bool,
    pub zap: bool,
    pub flash: bool,
    pub typing: bool,
    pub ripple: bool,
    pub confetti: u8,
    pub notes: Notes,
    pub poof: Poof,
    pub shadow: Shadow,
}

/// Every field at rest; the generated frames list only what differs.
const D: Frame = Frame {
    eyes: Eyes::Open,
    arms: Arms::Down,
    feet: Feet::Both,
    offset: 0,
    x: 0.0,
    angle: 0.0,
    lift: 0.0,
    flip: 0.0,
    roll: 0.0,
    squat: 0.0,
    lean: 0.0,
    kick: 0.0,
    strum: 0.0,
    bang: 0.0,
    guitar: 0.0,
    laptop: 0.0,
    dumbbell: 0.0,
    curl_l: 0.0,
    curl_r: 0.0,
    fret: f64::NAN,
    windmill: false,
    zap: false,
    flash: false,
    typing: false,
    ripple: false,
    confetti: 0,
    notes: Notes::None,
    poof: Poof::None,
    shadow: Shadow::None,
};

pub struct Move {
    pub name: &'static str,
    pub frames: &'static [Frame],
}

/// Camera framing for a mode: what it looks at, from where, and how much must fit.
#[derive(Clone, Copy, Debug)]
pub struct Framing {
    pub target: [f64; 3],
    pub theta: f64,
    pub phi: f64,
    pub half_h: f64,
    pub half_w: f64,
}

#[allow(clippy::approx_constant)] // the data has values like 0.707107 that aren't meant as constants
mod data {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/moves.rs"));
}
pub use data::*;

/// A move's index by name.
pub fn find(name: &str) -> Option<usize> {
    MOVES.iter().position(|m| m.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exported_moves_are_all_here() {
        assert_eq!(MOVES.len(), 21);
        assert_eq!(MOVES[0].name, "jump");
        assert!(find("ultracode").is_some() && find("dance").is_none());
        let laptop = &MOVES[find("laptop").unwrap()];
        assert!(laptop.frames[TYPING_FROM].typing && laptop.frames[TYPING_TO].typing);
        assert!(!laptop.frames[TYPING_TO + 1].typing);
        assert!(laptop.frames[LAPTOP_OPEN].ripple);
        assert_eq!(IDLE.len(), 22);
        assert!(ON_CLICK.iter().all(|n| find(n).is_some()));
        assert_eq!(FRAME_MS, 60.0);
    }
}
