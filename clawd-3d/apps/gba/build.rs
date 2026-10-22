//! Draws Clawd's moves when the ROM is built, since a 16 MHz GBA can't run the 3D engine: clawd-core plays each move
//! from him standing still and clawd-raster draws it at 240×160, 15 frames a second, on the apps' dark stage. The
//! pictures share one 256-colour palette (the GBA's 8-bit bitmap mode), and each is stored as runs of the pixels that
//! differ from him standing still. Writes data.rs, and the .bin files it includes, to OUT_DIR for src/main.rs.
use clawd_core::moves::{FRAME_MS, IDLE, ON_CLICK};
use clawd_core::textures::glyph_bits;
use clawd_core::{Clawd, Mode, Options};
use clawd_raster::{Background, Renderer, Settings};
use std::fmt::Write as _;
use std::path::PathBuf;

const W: usize = 240;
const H: usize = 160;
const STEP: f64 = 1.0 / 60.0;
/// Engine steps (1/60 s, as the page) per stored frame: 15 a second, each shown for four of the GBA's 60 Hz frames.
const STEPS: usize = 4;
const COLORS: usize = 256;
/// The most used colours, kept exactly: a near miss on the background would draw rings on it.
const PINNED: usize = 64;
/// The idle clip's frame the others are stored against: his eyes are back in the middle by then.
const BASE_AT: usize = 8;
/// The longest he may take to settle after a move before the ROM cuts back to idle anyway.
const MAX_TAIL: usize = 45;
/// Colours this dark (every channel) are dithered: the background, the shadows on the floor, his eyes.
const DITHER_BELOW: u8 = 44;

fn stage() -> (Clawd, Renderer) {
    let mut clawd = Clawd::new(Options { mode: Mode::Stage, width: W as f64, height: H as f64, seed: 10 });
    clawd.set_colors(0x2b3138, 0x4a525c, 0x4a525c, 0x4cc9db); // the apps' dark stage, as clawd-render
    clawd.update(0.0);
    let settings =
        Settings { background: Background::App, shadow_size: 2048, soft_shadows: true, supersample: 2, effects: true };
    (clawd, Renderer::new(W, H, settings))
}

/// The picture in the GBA's 15-bit colour (red in the low bits). The dark background has only a few steps of 15-bit
/// colour, and rounded they draw rings, each a different tint: dark pixels are dithered instead (always the same way
/// at the same spot, so the stage is the same in every frame).
fn shoot(clawd: &Clawd, renderer: &mut Renderer) -> Vec<u16> {
    const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let (pixels, _) = renderer.render(clawd).as_chunks::<4>();
    pixels
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let dark = p[..3].iter().all(|&c| c <= DITHER_BELOW);
            let threshold = if dark { (BAYER[i / W % 4][i % W % 4] as u32 * 2 + 1) * 255 / 32 } else { 127 };
            let five = |c: u8| ((c as u32 * 31 + threshold) / 255).min(31) as u16;
            five(p[0]) | five(p[1]) << 5 | five(p[2]) << 10
        })
        .collect()
}

fn rgb(c: u16) -> [i32; 3] {
    [(c & 31) as i32, (c >> 5 & 31) as i32, (c >> 10 & 31) as i32]
}

/// Pixels that differ by more than a step of 15-bit colour: the easing never quite lands, so exact can wait forever.
fn differ(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b).filter(|(x, y)| rgb(**x).iter().zip(rgb(**y)).any(|(p, q)| (p - q).abs() > 1)).count()
}

struct Clip {
    name: &'static str,
    frames: Vec<Vec<u16>>,
    /// The idle frame that follows the clip's last one.
    resume: usize,
}

/// One loop of his idle (eyes right, then left), from its second time round, so it starts as every later loop does.
fn idle() -> Clip {
    let (mut clawd, mut renderer) = stage();
    let loop_steps = IDLE.len() as f64 * FRAME_MS / 1000.0 / STEP;
    for _ in 0..loop_steps.round() as usize {
        clawd.update(STEP);
    }
    let mut frames = Vec::new();
    for _ in 0..(loop_steps / STEPS as f64).round() as usize {
        frames.push(shoot(&clawd, &mut renderer));
        for _ in 0..STEPS {
            clawd.update(STEP);
        }
    }
    Clip { name: "idle", frames, resume: 0 }
}

/// A move from him standing still (as a tap in the apps), until he has settled back into the idle loop.
fn play(name: &'static str, idle: &[Vec<u16>]) -> Clip {
    let (mut clawd, mut renderer) = stage();
    assert!(clawd.play(name), "no move called {name}");
    let mut frames = vec![shoot(&clawd, &mut renderer)];
    let mut idle_steps = None;
    loop {
        for _ in 0..STEPS {
            clawd.update(STEP);
            if let Some(s) = idle_steps.as_mut() {
                *s += 1;
            } else if clawd.current() == "idle" {
                idle_steps = Some(1);
            }
        }
        let frame = shoot(&clawd, &mut renderer);
        if let Some(s) = idle_steps {
            // Where the idle loop he is in now has got to: once he looks like that frame of the idle clip, cut to it.
            let at = (s as f64 / STEPS as f64).round() as usize % idle.len();
            let tail = s / STEPS;
            if (tail >= 3 && differ(&frame, &idle[at]) == 0) || tail >= MAX_TAIL {
                if tail >= MAX_TAIL {
                    println!("cargo:warning={name} never settled; {} pixels off", differ(&frame, &idle[at]));
                }
                return Clip { name, frames, resume: at };
            }
        }
        frames.push(frame);
    }
}

/// The palette for every frame: the most used colours exactly (the stage's gradient and floor, his lit faces), and for
/// the rest a median cut weighted by the square root of how often each is used, so his sparks and the guitar wall get
/// a say next to the background, then a few rounds of k-means. Returns the palette and each 15-bit colour's entry.
fn palette(clips: &[Clip]) -> (Vec<u16>, Vec<u8>) {
    let mut counts = vec![0u64; 1 << 15];
    for c in clips.iter().flat_map(|c| &c.frames).flatten() {
        counts[*c as usize] += 1;
    }
    let mut used: Vec<(u16, f64)> =
        (0..1 << 15).filter(|&c| counts[c] > 0).map(|c| (c as u16, (counts[c] as f64).sqrt())).collect();
    used.sort_by(|a, b| b.1.total_cmp(&a.1));
    let point = |c: u16| rgb(c).map(|v| v as f64);
    let pinned = PINNED.min(used.len());
    let mut boxes: Vec<Vec<usize>> = vec![(pinned..used.len()).collect()];
    boxes.retain(|b| !b.is_empty());
    while pinned + boxes.len() < COLORS {
        // The box with the most weight times spread splits at its weighted median, along its longest side.
        let spread = |b: &Vec<usize>| {
            let range = |axis: usize| {
                let values = b.iter().map(|&i| point(used[i].0)[axis]);
                values.clone().fold(f64::MIN, f64::max) - values.fold(f64::MAX, f64::min)
            };
            (0..3).map(|axis| (range(axis), axis)).max_by(|a, b| a.0.total_cmp(&b.0)).unwrap()
        };
        let weight = |b: &Vec<usize>| b.iter().map(|&i| used[i].1).sum::<f64>();
        let Some((k, axis)) = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .map(|(k, b)| (k, spread(b), weight(b)))
            .max_by(|a, b| (a.1.0 * a.2).total_cmp(&(b.1.0 * b.2)))
            .map(|(k, (_, axis), _)| (k, axis))
        else {
            break;
        };
        let mut b = boxes.swap_remove(k);
        b.sort_by(|&i, &j| point(used[i].0)[axis].total_cmp(&point(used[j].0)[axis]));
        let (half, mut acc) = (weight(&b) / 2.0, 0.0);
        let cut = b.iter().position(|&i| {
            acc += used[i].1;
            acc >= half
        });
        let rest = b.split_off((cut.unwrap_or(0) + 1).clamp(1, b.len() - 1));
        boxes.push(b);
        boxes.push(rest);
    }
    let mean = |members: &mut dyn Iterator<Item = usize>| {
        let (mut sum, mut weight) = ([0.0; 3], 0.0);
        for i in members {
            let p = point(used[i].0);
            for a in 0..3 {
                sum[a] += p[a] * used[i].1;
            }
            weight += used[i].1;
        }
        sum.map(|s| s / weight.max(1e-9))
    };
    let mut centres: Vec<[f64; 3]> = (0..pinned).map(|i| point(used[i].0)).collect();
    centres.extend(boxes.iter().map(|b| mean(&mut b.iter().copied())));
    // Green counts most and blue least, as the eye sees them.
    let dist = |a: [f64; 3], b: [f64; 3]| {
        3.0 * (a[0] - b[0]).powi(2) + 4.0 * (a[1] - b[1]).powi(2) + 2.0 * (a[2] - b[2]).powi(2)
    };
    let nearest = |centres: &[[f64; 3]], p: [f64; 3]| {
        (0..centres.len()).min_by(|&i, &j| dist(centres[i], p).total_cmp(&dist(centres[j], p))).unwrap()
    };
    let mut owner = vec![0usize; used.len()];
    for _ in 0..6 {
        for (i, (c, _)) in used.iter().enumerate() {
            owner[i] = nearest(&centres, point(*c));
        }
        let mut members = vec![Vec::new(); centres.len()];
        for (i, &k) in owner.iter().enumerate() {
            members[k].push(i);
        }
        for k in pinned..centres.len() {
            if !members[k].is_empty() {
                centres[k] = mean(&mut members[k].iter().copied());
            }
        }
    }
    let mut colours: Vec<u16> = centres
        .iter()
        .map(|c| c[0].round() as u16 | (c[1].round() as u16) << 5 | (c[2].round() as u16) << 10)
        .collect();
    colours.resize(COLORS, 0);
    let mut lut = vec![0u8; 1 << 15];
    for (c, _) in &used {
        lut[*c as usize] = nearest(&centres, point(*c)) as u8;
    }
    (colours, lut)
}

/// One frame: the box around the pixels that differ from `base` (x and width in fours, for word copies), then each of
/// its rows as runs of pixel pairs, since VRAM is written 16 bits at a time: 0x00+n skips n+1 pairs (the stage's, put
/// back before each frame), 0x40+n repeats the next pair n+1 times, and 0x80+n copies the next n+1 pairs.
fn encode(frame: &[u8], base: &[u8], out: &mut Vec<u8>) {
    let changed: Vec<usize> = (0..W * H).filter(|&i| frame[i] != base[i]).collect();
    if changed.is_empty() {
        out.extend([0; 4]);
        return;
    }
    let x0 = changed.iter().map(|i| i % W).min().unwrap() & !3;
    let x1 = (changed.iter().map(|i| i % W).max().unwrap() + 4) & !3;
    let (y0, y1) = (changed[0] / W, changed[changed.len() - 1] / W + 1);
    out.extend([(x0 / 4) as u8, y0 as u8, ((x1 - x0) / 4) as u8, (y1 - y0) as u8]);
    for y in y0..y1 {
        let pair = |v: &[u8], x: usize| [v[y * W + x * 2], v[y * W + x * 2 + 1]];
        let (f, b) = (|x| pair(frame, x), |x| pair(base, x));
        let end = x1 / 2;
        let same = |x: usize| (x..end.min(x + 64)).take_while(|&i| f(i) == f(x)).count();
        let mut x = x0 / 2;
        while x < end {
            if f(x) == b(x) {
                let n = (x..end.min(x + 64)).take_while(|&i| f(i) == b(i)).count();
                out.push(n as u8 - 1);
                x += n;
            } else if same(x) >= 2 {
                let n = same(x);
                out.push(0x40 | (n as u8 - 1));
                out.extend(f(x));
                x += n;
            } else {
                let start = x;
                while x < end && x - start < 128 && f(x) != b(x) && same(x) < 2 {
                    x += 1;
                }
                out.push(0x80 | (x - start - 1) as u8);
                (start..x).for_each(|i| out.extend(f(i)));
            }
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let idle = idle();
    // The moves a tap plays in the apps: all but the ones that walk in from off the stage.
    let moves: Vec<Clip> = ON_CLICK.iter().map(|name| play(name, &idle.frames)).collect();
    let clips: Vec<Clip> = std::iter::once(idle).chain(moves).collect();
    let (colours, lut) = palette(&clips);
    let index = |f: &Vec<u16>| f.iter().map(|c| lut[*c as usize]).collect::<Vec<u8>>();
    let base = index(&clips[0].frames[BASE_AT]);
    let (mut bytes, mut offsets, mut table) = (Vec::new(), Vec::new(), String::new());
    for clip in &clips {
        let first = offsets.len();
        for f in &clip.frames {
            offsets.push(bytes.len() as u32);
            encode(&index(f), &base, &mut bytes);
        }
        writeln!(
            table,
            "    Clip {{ name: {:?}, first: {first}, len: {}, resume: {} }},",
            clip.name,
            clip.frames.len(),
            clip.resume
        )
        .unwrap();
    }
    offsets.push(bytes.len() as u32);
    std::fs::write(out_dir.join("base.bin"), &base).unwrap();
    std::fs::write(out_dir.join("frames.bin"), &bytes).unwrap();

    let mut src = String::from("// Generated by build.rs: Clawd's moves, drawn by the Rust engine.\n");
    writeln!(src, "pub static PALETTE: [u16; 256] = {colours:?};").unwrap();
    writeln!(
        src,
        "pub static BASE: Aligned<[u8; {}]> = Aligned(*include_bytes!(concat!(env!(\"OUT_DIR\"), \"/base.bin\")));",
        W * H
    )
    .unwrap();
    writeln!(src, "pub static FRAMES: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/frames.bin\"));").unwrap();
    writeln!(src, "pub static OFFSETS: [u32; {}] = {offsets:?};", offsets.len()).unwrap();
    writeln!(src, "pub static CLIPS: [Clip; {}] = [\n{table}];", clips.len()).unwrap();
    let font: Vec<[u8; 8]> = (b' '..=b'~').map(glyph_bits).collect();
    writeln!(src, "pub static FONT: [[u8; 8]; {}] = {font:?};", font.len()).unwrap();
    std::fs::write(out_dir.join("data.rs"), src).unwrap();
}
