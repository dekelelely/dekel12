//! Clawd's model, sizes and colours as clawd-engine.js builds it: boxes from the terminal sprite's columns, a laptop,
//! an electric guitar and two dumbbells, a floor strip with the welcome screen's dotted line, and the lights.
use crate::math::{lin, v3, Vec3};

pub const COL: f64 = 2.0; // one terminal column is two sprite quadrants wide
pub const BODY_W: f64 = 12.0;
pub const BODY_H: f64 = 9.0;
pub const LEG_H: f64 = 2.8;
pub const DEPTH: f64 = 5.5;
pub const EYE: f64 = 1.4;
pub const ARM_LEN: f64 = 2.7;
pub const LEG_XS: [f64; 4] = [-4.5, -2.5, 2.5, 4.5];
pub const EYE_XS: [f64; 2] = [-3.5, 3.5];
/// Offset 2 leaves only the eyes above the floor line.
pub const SINK: [f64; 4] = [0.0, 0.0, LEG_H + BODY_H / 2.0, LEG_H + BODY_H + 1.0];

/// The laptop: base height puts the keys just under his level hands.
pub const LT_W: f64 = 8.5;
pub const LT_D: f64 = 5.0;
pub const LT_BASE: f64 = 1.75;
pub const LT_LID: f64 = 0.25;

/// The guitar is held low across his front, body over his right legs, neck rising past his left shoulder.
pub const GUITAR_X: f64 = 0.8;
pub const GUITAR_Y: f64 = 0.6;
pub const GUITAR_TILT: f64 = 0.9;

/// How a surface is lit: MeshStandardMaterial (lit, with roughness and metalness), MeshBasicMaterial (its colour
/// as it is), or ShadowMaterial (only the shadows falling on it, as see-through black).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shading {
    Standard { roughness: f32, metalness: f32 },
    Basic,
    Shadow { opacity: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// Linear-light colour.
    pub color: [f32; 3],
    pub shading: Shading,
}

const fn std(color: [f32; 3], roughness: f32, metalness: f32) -> Material {
    Material { color, shading: Shading::Standard { roughness, metalness } }
}

/// The materials, linear colours computed once (lin isn't const).
pub struct Materials {
    pub clay: Material,
    pub leg: Material,
    pub eye: Material,
    pub laptop: Material,
    pub keys: Material,
    pub dark: Material,
    pub lavender: Material,
    pub board: Material,
    pub string: Material,
    pub iron: Material,
    pub bar: Material,
    pub floor: Material,
    pub dots: Material,
    pub dust: Material,
    pub white: Material,
}

impl Materials {
    pub fn new() -> Materials {
        Materials {
            clay: std(lin(0xd77757), 0.62, 0.0),
            leg: std(lin(0xc86a4b), 0.7, 0.0),
            eye: std(lin(0x1c1a19), 0.4, 0.0),
            laptop: std(lin(0xd5d8dd), 0.45, 0.15),
            keys: std(lin(0x3a3f46), 0.8, 0.0),
            dark: std(lin(0x23202c), 0.55, 0.0),
            lavender: std(lin(0xa49cf0), 0.4, 0.0),
            board: std(lin(0x7d74d6), 0.6, 0.0),
            string: std(lin(0xe4e0ff), 0.3, 0.6),
            iron: std(lin(0x4a505a), 0.5, 0.3),
            bar: std(lin(0xd3d7de), 0.3, 0.6),
            floor: std(lin(0xb8bec6), 0.9, 0.0),
            dots: std(lin(0x8d949e), 0.9, 0.0),
            dust: std(lin(0xa9b0ba), 1.0, 0.0),
            white: Material { color: [1.0, 1.0, 1.0], shading: Shading::Basic },
        }
    }
}

impl Default for Materials {
    fn default() -> Self {
        Self::new()
    }
}

pub const SPARK_COLORS: [u32; 2] = [0xffd23f, 0xffe680];
pub const CONFETTI_COLORS: [u32; 6] = [0xffd23f, 0x4cc9db, 0xff6fa8, 0xa49cf0, 0x7fe0a0, 0xd77757];

/// Which guitar material a part uses.
#[derive(Clone, Copy, Debug)]
pub enum GuitarMat {
    Dark,
    Lavender,
    Board,
    String,
}

/// The guitar's boxes: size, material and spot in the guitar's own frame (neck along +x, strings facing +z).
pub fn guitar_parts() -> impl Iterator<Item = ([f64; 3], GuitarMat, Vec3)> {
    use GuitarMat::*;
    let fixed: [([f64; 3], GuitarMat, Vec3); 10] = [
        ([3.8, 4.2, 0.8], Dark, v3(0.0, 0.0, 0.0)),      // body
        ([1.3, 1.1, 0.8], Dark, v3(2.3, 1.45, 0.0)),     // upper horn
        ([1.0, 1.0, 0.8], Dark, v3(2.15, -1.5, 0.0)),    // lower horn
        ([3.0, 3.4, 0.1], Lavender, v3(0.1, 0.1, 0.44)), // lavender face
        ([0.32, 1.5, 0.14], Dark, v3(-0.8, 0.0, 0.52)),  // pickups
        ([0.32, 1.5, 0.14], Dark, v3(0.0, 0.0, 0.52)),
        ([0.4, 1.7, 0.16], String, v3(-1.3, 0.0, 0.5)),   // bridge
        ([6.6, 0.75, 0.42], Dark, v3(5.0, 0.0, 0.12)),    // neck
        ([6.0, 0.6, 0.08], Board, v3(4.9, 0.0, 0.37)),    // fretboard
        ([1.7, 1.1, 0.32], Lavender, v3(9.1, 0.1, 0.08)), // headstock
    ];
    let pegs = (0..3).flat_map(|i| {
        let x = 8.5 + i as f64 * 0.55;
        [([0.3, 0.3, 0.3], Dark, v3(x, 0.85, 0.08)), ([0.3, 0.3, 0.3], Dark, v3(x, -0.65, 0.08))]
    });
    let frets = (0..7).map(|i| ([0.06, 0.62, 0.1], String, v3(2.3 + i as f64 * 0.85, 0.0, 0.42)));
    let strings = [-0.2, -0.07, 0.07, 0.2].into_iter().map(|y| ([10.2, 0.04, 0.04], String, v3(3.6, y, 0.54)));
    fixed.into_iter().chain(pegs).chain(frets).chain(strings)
}

/// The floor's dotted line: x of each dot, built by the same repeated addition as the engine.
pub fn dot_xs() -> impl Iterator<Item = f64> {
    let mut x = -44.0;
    core::iter::from_fn(move || {
        if x <= 44.0 {
            let here = x;
            x += 1.6;
            Some(here)
        } else {
            None
        }
    })
}

/// A directional light: colour × intensity (linear), and the direction toward it.
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub color: [f32; 3],
    pub dir: Vec3,
}

pub struct Lights {
    /// Hemisphere light: sky and ground colours × intensity.
    pub sky: [f32; 3],
    pub ground: [f32; 3],
    pub sun: Light,
    pub fill: Light,
    pub rim: Light,
    pub sun_position: Vec3,
    pub sun_target: Vec3,
}

fn scaled(c: [f32; 3], s: f32) -> [f32; 3] {
    [c[0] * s, c[1] * s, c[2] * s]
}

impl Lights {
    pub fn new() -> Lights {
        let (sun_position, sun_target) = (v3(14.0, 26.0, 18.0), v3(-6.0, 3.0, 0.0));
        Lights {
            sky: scaled(lin(0xffffff), 0.5),
            ground: scaled(lin(0x6f625a), 0.5),
            sun: Light { color: scaled(lin(0xfff3e8), 0.95), dir: (sun_position - sun_target).normalize() },
            fill: Light { color: scaled(lin(0xcbd8ff), 0.35), dir: v3(-18.0, 10.0, -4.0).normalize() },
            rim: Light { color: scaled(lin(0xffffff), 0.35), dir: v3(0.0, 14.0, -22.0).normalize() },
            sun_position,
            sun_target,
        }
    }
}

impl Default for Lights {
    fn default() -> Self {
        Self::new()
    }
}

/// The sun's shadow camera (orthographic, as the engine sets it up).
pub const SHADOW_LEFT: f64 = -32.0;
pub const SHADOW_RIGHT: f64 = 24.0;
pub const SHADOW_TOP: f64 = 24.0;
pub const SHADOW_BOTTOM: f64 = -24.0;
pub const SHADOW_NEAR: f64 = 1.0;
pub const SHADOW_FAR: f64 = 90.0;
pub const SHADOW_BIAS: f64 = -0.0006;
pub const SHADOW_NORMAL_BIAS: f64 = 0.03;

#[cfg(test)]
mod tests {
    #[test]
    fn same_part_counts_as_the_engine() {
        assert_eq!(super::guitar_parts().count(), 10 + 6 + 7 + 4);
        assert_eq!(super::dot_xs().count(), 55); // not 56: the repeated + 1.6 overshoots 44, in JavaScript too
    }
}
