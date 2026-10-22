//! Clawd's engine without a browser: the moves of clawd-engine.js (exported to moves.json), played frame by frame,
//! and his model posed for each moment. `no_std` (with `alloc`), so it runs anywhere from a desktop to a Switch;
//! clawd-raster draws what it poses.
#![no_std]

extern crate alloc;

mod clawd;
pub mod math;
pub mod model;
pub mod moves;
pub mod textures;

pub use clawd::{Camera, Clawd, DrawBox, DrawQuad, Event, Mode, Note, Options, Rect, Ripple, Rng, Scene, Surface};
