//! Draws Clawd into pixels on the CPU, the way three.js draws him on the web page: lit boxes (MeshStandardMaterial's
//! hemisphere light, three directional lights with GGX highlights, and the sun's soft shadow map), the floor clip
//! that lets him sink into it, the laptop's screen, the guitar's see-through pixel wall, and the code ripple and
//! music notes drawn over the top. `no_std`, so the same renderer runs on a Switch.
#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use clawd_core::math::{v3, Mat4, Vec3};
use clawd_core::model::{self, Lights, Shading};
use clawd_core::textures::{glyph_at, glyph_bits, hash, Texture, NOTE_SHAPES, Z_SHAPE};
use clawd_core::{Clawd, DrawBox, DrawQuad, NoteKind, Ripple, Scene, Surface};
use core::f32::consts::PI;
use libm::{ceilf, floorf, powf, sqrtf};

/// What is behind him.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Background {
    /// Nothing: a floating window shows the desktop through.
    Transparent,
    Color([u8; 3]),
    /// The apps' full view: a dark radial gradient, lighter above the middle.
    App,
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub background: Background,
    /// The sun's shadow map, in texels on a side (the web page uses 2048, or 1024 floating).
    pub shadow_size: usize,
    /// three.js's soft shadows (16 samples); off, 4 samples blended.
    pub soft_shadows: bool,
    /// Draw at this many times the size and average down: smoother edges, at the square of the cost.
    pub supersample: usize,
    /// The code ripple and the music notes.
    pub effects: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { background: Background::App, shadow_size: 1024, soft_shadows: true, supersample: 1, effects: true }
    }
}

const NA: usize = 12; // per-vertex values: base light rgb, sun rgb, shadow map xyz, world y, texture u v
const A_SUN: usize = 3;
const A_SHADOW: usize = 6;
const A_WORLD_Y: usize = 9;
const A_U: usize = 10;

#[derive(Clone, Copy, Default)]
struct ClipVertex {
    pos: [f32; 4],
    a: [f32; NA],
}

/// A vertex on screen: pixel x, y, depth 0…1, 1/w, and the values divided by w (they interpolate linearly).
#[derive(Clone, Copy, Default)]
struct ScreenVertex {
    x: f32,
    y: f32,
    z: f32,
    iw: f32,
    a: [f32; NA],
}

#[derive(Clone, Copy, PartialEq)]
enum Fill {
    /// Lit or flat colour, written whole.
    Solid { receive_shadow: bool, clip: bool },
    /// A texture as it is, opaque (the laptop's screen).
    Screen { clip: bool },
    /// A texture blended over what is there, without writing depth (the pixel wall).
    Wall { opacity: f32, clip: bool },
    /// Black where shadows fall, see-through (pet mode's floor).
    ShadowCatcher { opacity: f32 },
    /// The sun's depth, for the shadow map.
    Depth,
}

/// A mip chain of a texture, so the small laptop screen isn't a shimmer of text.
struct Mips {
    version: u32,
    levels: Vec<(usize, usize, Vec<[f32; 4]>)>,
}

impl Mips {
    fn new() -> Mips {
        Mips { version: u32::MAX, levels: Vec::new() }
    }

    /// Rebuilds the chain when the texture changed, up to `max_levels` levels.
    fn update(&mut self, t: &Texture, max_levels: usize) {
        if self.version == t.version && !self.levels.is_empty() {
            return;
        }
        self.version = t.version;
        self.levels.clear();
        let base: Vec<[f32; 4]> = t
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0])
            .collect();
        self.levels.push((t.width, t.height, base));
        while self.levels.last().is_some_and(|(w, h, _)| *w > 1 && *h > 1) && self.levels.len() < max_levels {
            let (w, h, src) = self.levels.last().unwrap();
            let (nw, nh) = (w / 2, h / 2);
            let mut dst = Vec::with_capacity(nw * nh);
            for y in 0..nh {
                for x in 0..nw {
                    let mut s = [0.0; 4];
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let p = src[(y * 2 + dy) * w + x * 2 + dx];
                        for k in 0..4 {
                            s[k] += p[k] * 0.25;
                        }
                    }
                    dst.push(s);
                }
            }
            self.levels.push((nw, nh, dst));
        }
    }

    /// Bilinear (or nearest) sample of mip level `lod`; v runs down the picture.
    fn sample(&self, u: f32, v: f32, lod: usize, nearest: bool) -> [f32; 4] {
        let (w, h, px) = &self.levels[lod.min(self.levels.len() - 1)];
        let (w, h) = (*w, *h);
        let fx = u.clamp(0.0, 1.0) * w as f32 - 0.5;
        let fy = v.clamp(0.0, 1.0) * h as f32 - 0.5;
        if nearest {
            let x = ((fx + 0.5) as usize).min(w - 1);
            let y = ((fy + 0.5) as usize).min(h - 1);
            return px[y * w + x];
        }
        let (x0, y0) = (floorf(fx), floorf(fy));
        let (tx, ty) = (fx - x0, fy - y0);
        let at =
            |x: f32, y: f32| px[(y.clamp(0.0, (h - 1) as f32) as usize) * w + x.clamp(0.0, (w - 1) as f32) as usize];
        let (a, b, c, d) = (at(x0, y0), at(x0 + 1.0, y0), at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0));
        let mut out = [0.0; 4];
        for k in 0..4 {
            out[k] = (a[k] * (1.0 - tx) + b[k] * tx) * (1.0 - ty) + (c[k] * (1.0 - tx) + d[k] * tx) * ty;
        }
        out
    }
}

pub struct Renderer {
    settings: Settings,
    width: usize,
    height: usize,
    /// Drawn at supersample × the size: sRGB colour (RGBA, not premultiplied) and depth.
    color: Vec<[u8; 4]>,
    depth: Vec<f32>,
    out: Vec<u8>,
    shadow: Vec<f32>,
    /// The part of the shadow map the casters cover (texels: x0, y0, x1, y1), last frame and this one: outside it
    /// nothing is in shadow, and only it needs clearing.
    shadow_box: [usize; 4],
    /// For each texel near the casters, the nearest and farthest depth among the 16 texels a soft-shadow lookup
    /// based there reads: a point nearer than all of them is lit, farther than all of them shadowed, at one read.
    footprint: Vec<(f32, f32)>,
    footprint_box: [isize; 4],
    shadow_matrix: Mat4,
    shadow_vp: Mat4,
    scene: Scene,
    /// The background, drawn once per size.
    backdrop: Vec<[u8; 4]>,
    backdrop_key: (usize, usize, u8),
    screen: Mips,
    wall: Mips,
    srgb: Vec<u8>,
}

/// three.js's LinearTosRGB.
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        powf(c, 0.41666) * 1.055 - 0.055
    }
}

const LUT: usize = 4096;

impl Renderer {
    pub fn new(width: usize, height: usize, settings: Settings) -> Renderer {
        let srgb = (0..LUT)
            .map(|i| (linear_to_srgb(i as f32 / (LUT - 1) as f32).clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
            .collect();
        let mut r = Renderer {
            settings,
            width: 0,
            height: 0,
            color: Vec::new(),
            depth: Vec::new(),
            out: Vec::new(),
            shadow: vec![1.0; settings.shadow_size * settings.shadow_size],
            shadow_box: [0, 0, settings.shadow_size, settings.shadow_size],
            footprint: Vec::new(),
            footprint_box: [0, 0, 0, 0],
            shadow_matrix: Mat4::IDENTITY,
            shadow_vp: Mat4::IDENTITY,
            scene: Scene::default(),
            backdrop: Vec::new(),
            backdrop_key: (0, 0, 0),
            screen: Mips::new(),
            wall: Mips::new(),
            srgb,
        };
        r.resize(width, height);
        r
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width.max(1);
        self.height = height.max(1);
        let ss = self.settings.supersample.max(1);
        let n = self.width * ss * self.height * ss;
        self.color = vec![[0; 4]; n];
        self.depth = vec![1.0; n];
        self.out = vec![0; self.width * self.height * 4];
    }

    pub fn settings(&self) -> Settings {
        self.settings
    }

    pub fn set_background(&mut self, background: Background) {
        self.settings.background = background;
    }

    fn encode(&self, linear: f32) -> u8 {
        self.srgb[((linear.clamp(0.0, 1.0) * (LUT - 1) as f32) + 0.5) as usize]
    }

    /// Draws him as he is now; returns the picture, RGBA (not premultiplied), `width × height`.
    pub fn render(&mut self, clawd: &Clawd) -> &[u8] {
        let ss = self.settings.supersample.max(1);
        let (w, h) = (self.width * ss, self.height * ss);
        let mut scene = core::mem::take(&mut self.scene);
        clawd.scene(&mut scene);
        self.screen.update(&clawd.screen, 6);
        // The wall changes every frame and is always seen larger than its cells: its first level is all it needs.
        self.wall.update(&clawd.wall, 1);
        self.clear(w, h);
        self.shadow_pass(&scene, clawd.lights());
        let camera = clawd.camera();
        let vp = camera.view_projection();
        let lights = clawd.lights();
        for b in &scene.boxes {
            self.draw_box(b, &vp, camera.position, lights, w, h);
        }
        // The laptop's screen is opaque; the see-through pictures go last, farthest first.
        let (opaque, mut clear): (Vec<&DrawQuad>, Vec<&DrawQuad>) =
            scene.quads.iter().partition(|q| q.surface == Surface::Screen);
        for q in opaque {
            self.draw_quad(q, &vp, w, h);
        }
        clear.sort_by(|a, b| {
            let da = (a.world.origin() - camera.position).length();
            let db = (b.world.origin() - camera.position).length();
            db.partial_cmp(&da).unwrap_or(core::cmp::Ordering::Equal)
        });
        for q in clear {
            self.draw_quad(q, &vp, w, h);
        }
        self.scene = scene;
        if self.settings.effects {
            self.draw_effects(clawd, w, h, ss);
        }
        self.resolve(ss);
        &self.out
    }

    fn clear(&mut self, w: usize, h: usize) {
        self.depth.fill(1.0);
        match self.settings.background {
            Background::Transparent => self.color.fill([0; 4]),
            Background::Color(c) => self.color.fill([c[0], c[1], c[2], 255]),
            Background::App => {
                if self.backdrop_key != (w, h, 1) {
                    // radial-gradient(120% 90% at 50% 30%, #1e2227, #0d0f12), as the page's full view
                    self.backdrop_key = (w, h, 1);
                    self.backdrop.clear();
                    let (hi, lo) = ([0x1e, 0x22, 0x27], [0x0d, 0x0f, 0x12]);
                    let (cx, cy, rx, ry) = (w as f32 * 0.5, h as f32 * 0.3, w as f32 * 1.2, h as f32 * 0.9);
                    for y in 0..h {
                        let dy = (y as f32 + 0.5 - cy) / ry;
                        for x in 0..w {
                            let dx = (x as f32 + 0.5 - cx) / rx;
                            let t = sqrtf(dx * dx + dy * dy).min(1.0);
                            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t + 0.5) as u8;
                            self.backdrop.push([mix(hi[0], lo[0]), mix(hi[1], lo[1]), mix(hi[2], lo[2]), 255]);
                        }
                    }
                }
                self.color.copy_from_slice(&self.backdrop);
            }
        }
    }

    /// Averages the supersampled picture down into `out`.
    fn resolve(&mut self, ss: usize) {
        let (w, h, sw) = (self.width, self.height, self.width * ss);
        if ss == 1 {
            for (o, c) in self.out.as_chunks_mut::<4>().0.iter_mut().zip(self.color.iter()) {
                o.copy_from_slice(c);
            }
            return;
        }
        let n = (ss * ss) as u32;
        for y in 0..h {
            for x in 0..w {
                let mut s = [0u32; 4];
                for dy in 0..ss {
                    for dx in 0..ss {
                        let c = self.color[(y * ss + dy) * sw + x * ss + dx];
                        for k in 0..4 {
                            s[k] += c[k] as u32;
                        }
                    }
                }
                let o = &mut self.out[(y * w + x) * 4..][..4];
                for k in 0..4 {
                    o[k] = ((s[k] + n / 2) / n) as u8;
                }
            }
        }
    }

    // ---- The sun's shadow map ----

    fn shadow_pass(&mut self, scene: &Scene, lights: &Lights) {
        let size = self.settings.shadow_size;
        let view =
            Mat4::look_at(lights.sun_position, lights.sun_target, v3(0., 1., 0.)).invert().unwrap_or(Mat4::IDENTITY);
        let proj = Mat4::orthographic(
            model::SHADOW_LEFT,
            model::SHADOW_RIGHT,
            model::SHADOW_TOP,
            model::SHADOW_BOTTOM,
            model::SHADOW_NEAR,
            model::SHADOW_FAR,
        );
        self.shadow_vp = proj * view;
        // To shadow map coordinates: x, y and depth all 0…1.
        let bias = Mat4([0.5, 0., 0., 0., 0., 0.5, 0., 0., 0., 0., 0.5, 0., 0.5, 0.5, 0.5, 1.]);
        self.shadow_matrix = bias * self.shadow_vp;
        // Clear what last frame's casters covered (the rest of the map is still clear).
        let [x0, y0, x1, y1] = self.shadow_box;
        for y in y0..y1 {
            self.shadow[y * size + x0..y * size + x1].fill(1.0);
        }
        let mut cover = [usize::MAX, usize::MAX, 0, 0];
        for b in scene.boxes.iter().filter(|b| b.cast_shadow) {
            for face in faces(&b.world) {
                // three.js draws the casters' back faces into the shadow map
                let pts: [ClipVertex; 4] = face.corners.map(|p| clip_vertex(&self.shadow_vp, p, [0.0; NA]));
                for tri in [[pts[0], pts[1], pts[2]], [pts[0], pts[2], pts[3]]] {
                    let s = to_screen(&tri, size, size);
                    if signed_area(&s) < 0.0 {
                        for p in &s {
                            let (x, y) = (p.x.clamp(0.0, size as f32) as usize, p.y.clamp(0.0, size as f32) as usize);
                            cover = [cover[0].min(x), cover[1].min(y), cover[2].max(x + 1), cover[3].max(y + 1)];
                        }
                        self.raster(&s, Fill::Depth, size, size);
                    }
                }
            }
        }
        self.shadow_box = if cover[0] == usize::MAX {
            [0, 0, 0, 0]
        } else {
            [cover[0].saturating_sub(1), cover[1].saturating_sub(1), (cover[2] + 1).min(size), (cover[3] + 1).min(size)]
        };
        self.build_footprint();
    }

    /// The footprint map over the casters' box and 3 texels around it. A lookup based at column X, row R reads
    /// columns X−1…X+2 and rows R−2…R+1; texels off the map count as lit (infinitely far).
    fn build_footprint(&mut self) {
        let size = self.settings.shadow_size as isize;
        let [x0, y0, x1, y1] = self.shadow_box.map(|v| v as isize);
        if x1 <= x0 || y1 <= y0 {
            self.footprint_box = [0, 0, 0, 0];
            return;
        }
        let (bx0, by0, bx1, by1) = (x0 - 3, y0 - 3, x1 + 3, y1 + 3);
        let (bw, bh) = ((bx1 - bx0) as usize, (by1 - by0) as usize);
        let at = |x: isize, y: isize| -> f32 {
            if x < 0 || y < 0 || x >= size || y >= size {
                f32::INFINITY
            } else {
                self.shadow[(y * size + x) as usize]
            }
        };
        // across: rows by0−2 … by1+1
        let rows = bh + 3;
        let mut across = vec![(0.0f32, 0.0f32); rows * bw];
        for r in 0..rows {
            let y = by0 - 2 + r as isize;
            for c in 0..bw {
                let x = bx0 + c as isize;
                let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
                for dx in -1..=2 {
                    let d = at(x + dx, y);
                    lo = lo.min(d);
                    hi = hi.max(d);
                }
                across[r * bw + c] = (lo, hi);
            }
        }
        // then down
        self.footprint.clear();
        self.footprint.reserve(bw * bh);
        for r in 0..bh {
            for c in 0..bw {
                let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
                for dr in 0..4 {
                    let (l, h) = across[(r + dr) * bw + c];
                    lo = lo.min(l);
                    hi = hi.max(h);
                }
                self.footprint.push((lo, hi));
            }
        }
        self.footprint_box = [bx0, by0, bx1, by1];
    }

    /// How lit by the sun a point is, 0…1, from its shadow map coordinates (three.js's PCF soft shadows).
    fn shadow_at(&self, sx: f32, sy: f32, sz: f32) -> f32 {
        if !(0.0..=1.0).contains(&sx) || !(0.0..=1.0).contains(&sy) || sz > 1.0 {
            return 1.0;
        }
        let size = self.settings.shadow_size;
        let sizef = size as f32;
        // Away from where the casters are, nothing is in shadow: no need to look (most of the floor).
        let [x0, y0, x1, y1] = self.shadow_box;
        let (tx, ty) = (sx * sizef, (1.0 - sy) * sizef);
        if tx < x0 as f32 - 3.0 || tx > x1 as f32 + 3.0 || ty < y0 as f32 - 3.0 || ty > y1 as f32 + 3.0 {
            return 1.0;
        }
        let texel = 1.0 / sizef;
        // Wholly lit or wholly shadowed under every texel the lookup reads: answered at once.
        // (sx and sy are 0…1 here, so truncating is flooring)
        let base_x = (sx * sizef + 0.5) as isize - 1;
        let base_r = size as isize - (sy * sizef + 0.5) as isize;
        let [bx0, by0, bx1, by1] = self.footprint_box;
        if base_x >= bx0 && base_x < bx1 && base_r >= by0 && base_r < by1 {
            let (lo, hi) = self.footprint[((base_r - by0) * (bx1 - bx0) + (base_x - bx0)) as usize];
            if sz <= lo {
                return 1.0;
            }
            if sz > hi {
                return 0.0;
            }
        }
        let cmp = |u: f32, v: f32| -> f32 {
            let x = (u * sizef) as isize;
            let y = ((1.0 - v) * sizef) as isize;
            if x < 0 || y < 0 || x >= size as isize || y >= size as isize {
                return 1.0;
            }
            if sz <= self.shadow[y as usize * size + x as usize] {
                1.0
            } else {
                0.0
            }
        };
        let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let fx = sx * sizef + 0.5;
        let fy = sy * sizef + 0.5;
        let (f0, f1) = (fx - (fx as i32) as f32, fy - (fy as i32) as f32);
        let (u, v) = (sx - f0 * texel, sy - f1 * texel);
        let (dx, dy) = (texel, texel);
        if !self.settings.soft_shadows {
            let a = mix(cmp(u, v), cmp(u + dx, v), f0);
            let b = mix(cmp(u, v + dy), cmp(u + dx, v + dy), f0);
            return mix(a, b, f1);
        }
        (cmp(u, v)
            + cmp(u + dx, v)
            + cmp(u, v + dy)
            + cmp(u + dx, v + dy)
            + mix(cmp(u - dx, v), cmp(u + 2.0 * dx, v), f0)
            + mix(cmp(u - dx, v + dy), cmp(u + 2.0 * dx, v + dy), f0)
            + mix(cmp(u, v - dy), cmp(u, v + 2.0 * dy), f1)
            + mix(cmp(u + dx, v - dy), cmp(u + dx, v + 2.0 * dy), f1)
            + mix(
                mix(cmp(u - dx, v - dy), cmp(u + 2.0 * dx, v - dy), f0),
                mix(cmp(u - dx, v + 2.0 * dy), cmp(u + 2.0 * dx, v + 2.0 * dy), f0),
                f1,
            ))
            * (1.0 / 9.0)
    }

    // ---- Boxes and pictures ----

    fn draw_box(&mut self, b: &DrawBox, vp: &Mat4, eye: Vec3, lights: &Lights, w: usize, h: usize) {
        for face in faces(&b.world) {
            let center = (face.corners[0] + face.corners[2]) * 0.5;
            let facing = face.normal.dot(eye - center) > 0.0;
            if !facing && !b.double_sided {
                continue;
            }
            let normal = if facing { face.normal } else { -face.normal };
            let verts: [ClipVertex; 4] = face.corners.map(|p| {
                let mut a = [0.0f32; NA];
                match b.material.shading {
                    Shading::Basic => a[..3].copy_from_slice(&b.material.color),
                    Shading::Standard { roughness, metalness } => {
                        let (base, sun) =
                            light(b.material.color, roughness, metalness, normal, (eye - p).normalize(), lights);
                        a[..3].copy_from_slice(&base);
                        a[A_SUN..A_SUN + 3].copy_from_slice(&sun);
                    }
                    Shading::Shadow { .. } => {}
                }
                let biased = p + normal * model::SHADOW_NORMAL_BIAS;
                let s = self.shadow_matrix.transform_point(biased);
                a[A_SHADOW] = s.x as f32;
                a[A_SHADOW + 1] = s.y as f32;
                a[A_SHADOW + 2] = (s.z + model::SHADOW_BIAS) as f32;
                a[A_WORLD_Y] = p.y as f32;
                clip_vertex(vp, p, a)
            });
            // Facing away from the sun, a face gets none of its light, and shadows can't take any away.
            let sunlit = verts.iter().any(|v| v.a[A_SUN] + v.a[A_SUN + 1] + v.a[A_SUN + 2] > 0.0);
            let fill = Fill::Solid {
                receive_shadow: sunlit && b.receive_shadow && matches!(b.material.shading, Shading::Standard { .. }),
                clip: b.clip,
            };
            for tri in [[verts[0], verts[1], verts[2]], [verts[0], verts[2], verts[3]]] {
                self.draw_clipped(&tri, fill, w, h);
            }
        }
    }

    fn draw_quad(&mut self, q: &DrawQuad, vp: &Mat4, w: usize, h: usize) {
        // The unit square in x, y facing +z; u runs right and v down the picture, as on a canvas.
        let corners = [(-0.5, -0.5, 0.0, 1.0), (0.5, -0.5, 1.0, 1.0), (0.5, 0.5, 1.0, 0.0), (-0.5, 0.5, 0.0, 0.0)];
        let verts = corners.map(|(x, y, u, v)| {
            let p = q.world.transform_point(v3(x, y, 0.0));
            let mut a = [0.0f32; NA];
            let s = self.shadow_matrix.transform_point(p + v3(0.0, 1.0, 0.0) * model::SHADOW_NORMAL_BIAS);
            a[A_SHADOW] = s.x as f32;
            a[A_SHADOW + 1] = s.y as f32;
            a[A_SHADOW + 2] = (s.z + model::SHADOW_BIAS) as f32;
            a[A_WORLD_Y] = p.y as f32;
            a[A_U] = u;
            a[A_U + 1] = v;
            clip_vertex(vp, p, a)
        });
        let fill = match q.surface {
            Surface::Screen => Fill::Screen { clip: q.clip },
            Surface::Wall => Fill::Wall { opacity: q.opacity, clip: q.clip },
            Surface::ShadowCatcher => Fill::ShadowCatcher { opacity: q.opacity },
        };
        for tri in [[verts[0], verts[1], verts[2]], [verts[0], verts[2], verts[3]]] {
            self.draw_clipped(&tri, fill, w, h);
        }
    }

    /// Clips a triangle against the near plane, then draws what is left.
    fn draw_clipped(&mut self, tri: &[ClipVertex; 3], fill: Fill, w: usize, h: usize) {
        const NEAR: f32 = 1e-4;
        if tri.iter().all(|v| v.pos[3] > NEAR) {
            let s = to_screen(tri, w, h);
            self.raster(&s, fill, w, h);
            return;
        }
        let mut poly: Vec<ClipVertex> = Vec::with_capacity(4);
        for i in 0..3 {
            let (a, b) = (tri[i], tri[(i + 1) % 3]);
            let (ia, ib) = (a.pos[3] > NEAR, b.pos[3] > NEAR);
            if ia {
                poly.push(a);
            }
            if ia != ib {
                let t = (NEAR - a.pos[3]) / (b.pos[3] - a.pos[3]);
                let mut v = ClipVertex::default();
                for k in 0..4 {
                    v.pos[k] = a.pos[k] + (b.pos[k] - a.pos[k]) * t;
                }
                for k in 0..NA {
                    v.a[k] = a.a[k] + (b.a[k] - a.a[k]) * t;
                }
                poly.push(v);
            }
        }
        for i in 1..poly.len().saturating_sub(1) {
            let s = to_screen(&[poly[0], poly[i], poly[i + 1]], w, h);
            self.raster(&s, fill, w, h);
        }
    }

    /// Fills a triangle row by row, every pixel whose centre is inside it (each shared edge drawn once).
    fn raster(&mut self, v: &[ScreenVertex; 3], fill: Fill, w: usize, h: usize) {
        let area = (v[1].x - v[0].x) * (v[2].y - v[0].y) - (v[2].x - v[0].x) * (v[1].y - v[0].y);
        if area.abs() < 1e-12 {
            return;
        }
        // Each value as a plane over the screen: value = base + dx·x + dy·y.
        let plane = |f0: f32, f1: f32, f2: f32| -> (f32, f32, f32) {
            let a = ((f1 - f0) * (v[2].y - v[0].y) - (f2 - f0) * (v[1].y - v[0].y)) / area;
            let b = ((f2 - f0) * (v[1].x - v[0].x) - (f1 - f0) * (v[2].x - v[0].x)) / area;
            (f0 - a * v[0].x - b * v[0].y, a, b)
        };
        let pz = plane(v[0].z, v[1].z, v[2].z);
        let piw = plane(v[0].iw, v[1].iw, v[2].iw);
        let mut pa = [(0.0f32, 0.0f32, 0.0f32); NA];
        let used: &[usize] = match fill {
            Fill::Depth => &[],
            Fill::Solid { receive_shadow: true, .. } => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            Fill::Solid { .. } => &[0, 1, 2, 9],
            Fill::Screen { .. } | Fill::Wall { .. } => &[9, 10, 11],
            Fill::ShadowCatcher { .. } => &[6, 7, 8],
        };
        for &k in used {
            pa[k] = plane(v[0].a[k], v[1].a[k], v[2].a[k]);
        }
        // Mip level for textures: how many texels a pixel covers.
        let lod = match fill {
            Fill::Screen { .. } | Fill::Wall { .. } => {
                let tex = if matches!(fill, Fill::Screen { .. }) { &self.screen } else { &self.wall };
                let (tw, th, _) = &tex.levels[0];
                let du = (v[1].a[A_U] / v[1].iw - v[0].a[A_U] / v[0].iw, v[2].a[A_U] / v[2].iw - v[0].a[A_U] / v[0].iw);
                let dv = (
                    v[1].a[A_U + 1] / v[1].iw - v[0].a[A_U + 1] / v[0].iw,
                    v[2].a[A_U + 1] / v[2].iw - v[0].a[A_U + 1] / v[0].iw,
                );
                let uv_area = (du.0 * dv.1 - du.1 * dv.0).abs() * *tw as f32 * *th as f32;
                let ratio = uv_area / area.abs();
                if ratio > 1.0 {
                    (libm::log2f(ratio) * 0.5) as usize
                } else {
                    0
                }
            }
            _ => 0,
        };
        let ymin = v.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let ymax = v.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
        let y0 = (ceilf(ymin - 0.5).max(0.0)) as usize;
        let y1 = (ceilf(ymax - 0.5).min(h as f32)).max(0.0) as usize;
        for y in y0..y1 {
            let yc = y as f32 + 0.5;
            // where the row crosses the edges
            let (mut xl, mut xr) = (f32::INFINITY, f32::NEG_INFINITY);
            for i in 0..3 {
                let (a, b) = (&v[i], &v[(i + 1) % 3]);
                if (a.y <= yc && yc < b.y) || (b.y <= yc && yc < a.y) {
                    let x = a.x + (yc - a.y) * (b.x - a.x) / (b.y - a.y);
                    xl = xl.min(x);
                    xr = xr.max(x);
                }
            }
            if xl >= xr {
                continue;
            }
            let x0 = ceilf(xl - 0.5).max(0.0) as usize;
            let x1 = (ceilf(xr - 0.5).min(w as f32)).max(0.0) as usize;
            for x in x0..x1 {
                let xc = x as f32 + 0.5;
                let z = pz.0 + pz.1 * xc + pz.2 * yc;
                let at = y * w + x;
                if let Fill::Depth = fill {
                    if z < self.shadow[at] {
                        self.shadow[at] = z;
                    }
                    continue;
                }
                if z > self.depth[at] || !(0.0..=1.0).contains(&z) {
                    continue;
                }
                let iw = piw.0 + piw.1 * xc + piw.2 * yc;
                let pw = 1.0 / iw;
                let val = |k: usize| (pa[k].0 + pa[k].1 * xc + pa[k].2 * yc) * pw;
                match fill {
                    Fill::Solid { receive_shadow, clip } => {
                        if clip && val(A_WORLD_Y) < 0.0 {
                            continue;
                        }
                        let mut c = [val(0), val(1), val(2)];
                        if receive_shadow {
                            let lit = self.shadow_at(val(A_SHADOW), val(A_SHADOW + 1), val(A_SHADOW + 2));
                            for (k, ck) in c.iter_mut().enumerate() {
                                *ck += val(A_SUN + k) * lit;
                            }
                        }
                        self.depth[at] = z;
                        self.color[at] = [self.encode(c[0]), self.encode(c[1]), self.encode(c[2]), 255];
                    }
                    Fill::Screen { clip } => {
                        if clip && val(A_WORLD_Y) < 0.0 {
                            continue;
                        }
                        let t = self.screen.sample(val(A_U), val(A_U + 1), lod, false);
                        self.depth[at] = z;
                        self.color[at] = [to_u8(t[0]), to_u8(t[1]), to_u8(t[2]), 255];
                    }
                    Fill::Wall { opacity, clip } => {
                        if clip && val(A_WORLD_Y) < 0.0 {
                            continue;
                        }
                        let t = self.wall.sample(val(A_U), val(A_U + 1), lod, lod == 0);
                        blend(&mut self.color[at], [t[0], t[1], t[2]], t[3] * opacity);
                    }
                    Fill::ShadowCatcher { opacity } => {
                        let lit = self.shadow_at(val(A_SHADOW), val(A_SHADOW + 1), val(A_SHADOW + 2));
                        blend(&mut self.color[at], [0.0, 0.0, 0.0], opacity * (1.0 - lit));
                        self.depth[at] = z;
                    }
                    Fill::Depth => {}
                }
            }
        }
    }

    // ---- The code ripple and music notes, over the top ----

    fn draw_effects(&mut self, clawd: &Clawd, w: usize, h: usize, ss: usize) {
        let (css_w, _) = clawd.size();
        let s = w as f32 / css_w as f32; // pixels per CSS pixel
        if let Some(r) = clawd.ripple() {
            self.draw_ripple(&r, clawd.code_color(), s, w, h);
        }
        let vp = clawd.camera().view_projection();
        let size = ((h / ss) as f32 / 110.0 + 0.5).max(2.0) as usize * ss;
        for note in clawd.notes() {
            let (p, _) = vp.project(note.pos);
            let (px, py) = ((p.x as f32 + 1.0) / 2.0 * w as f32, (1.0 - p.y as f32) / 2.0 * h as f32);
            let (color, shape) = match note.kind {
                NoteKind::Above => ([0xff, 0xd2, 0x3f], &NOTE_SHAPES[note.shape]),
                NoteKind::Side => ([0x8b, 0x8a, 0x9c], &NOTE_SHAPES[note.shape]),
                NoteKind::Z => ([0xc9, 0xd6, 0xff], &Z_SHAPE),
            };
            let alpha = (note.life as f32 * 2.0).min(1.0);
            for (r, row) in shape.iter().enumerate() {
                for (k, ch) in row.bytes().enumerate() {
                    if ch == b'#' {
                        let x = px + (k as f32 - 2.5) * size as f32 + 0.5;
                        let y = py + (r as f32 - 2.5) * size as f32 + 0.5;
                        self.fill_rect(x as isize, y as isize, size, size, color, alpha, w, h);
                    }
                }
            }
        }
    }

    /// Code that floods out from him: a growing disc of characters with a bright, crisp edge and dimmer ones behind.
    fn draw_ripple(&mut self, r: &Ripple, code: u32, s: f32, w: usize, h: usize) {
        let (rx, ry) = ((r.x as f32 + 1.0) / 2.0 * w as f32, (1.0 - r.y as f32) / 2.0 * h as f32);
        let (fx, fy) = (rx.max(w as f32 - rx), ry.max(h as f32 - ry));
        let far = sqrtf(fx * fx + fy * fy);
        let edge = Ripple::EDGE as f32 * s;
        let radius = r.radius(far as f64, edge as f64) as f32;
        let fade = r.fade() as f32;
        if fade <= 0.0 {
            return;
        }
        let color = [(code >> 16 & 255) as u8, (code >> 8 & 255) as u8, (code & 255) as u8];
        let (cw, ch) = (8.5 * s, 13.0 * s);
        let (cols, rows) = (ceilf(w as f32 / cw) as usize, ceilf(h as f32 / ch) as usize);
        let px = (s + 0.5).max(1.0) as usize; // one font pixel
        for j in 0..rows {
            let y = (j as f32 + 0.5) * ch;
            if (y - ry).abs() > radius {
                continue;
            }
            for i in 0..cols {
                let x = (i as f32 + 0.5) * cw;
                let d = sqrtf((x - rx) * (x - rx) + (y - ry) * (y - ry));
                if d > radius {
                    continue;
                }
                let rim = d > radius - edge && radius < far + edge;
                let Some(g) = glyph_at(i, j, r.t) else { continue };
                let shade = 0.55 + 0.45 * hash(i as f64, j as f64, 5.0) as f32;
                let alpha = fade * shade * if rim { 1.0 } else { Ripple::INSIDE as f32 };
                let bits = glyph_bits(g);
                let (gx, gy) = ((x - 4.0 * px as f32) as isize, (y - 4.0 * px as f32) as isize);
                for (row, b) in bits.iter().enumerate() {
                    for col in 0..8 {
                        if b & (1 << col) != 0 {
                            let (fx, fy) = (gx + (col * px) as isize, gy + (row * px) as isize);
                            self.fill_rect(fx, fy, px, px, color, alpha, w, h);
                            if rim {
                                self.fill_rect(fx + 1, fy, px, px, color, alpha, w, h);
                                // bold
                            }
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_rect(&mut self, x: isize, y: isize, rw: usize, rh: usize, rgb: [u8; 3], alpha: f32, w: usize, h: usize) {
        let c = [rgb[0] as f32 / 255.0, rgb[1] as f32 / 255.0, rgb[2] as f32 / 255.0];
        for yy in y.max(0)..(y + rh as isize).min(h as isize) {
            for xx in x.max(0)..(x + rw as isize).min(w as isize) {
                blend(&mut self.color[yy as usize * w + xx as usize], c, alpha);
            }
        }
    }
}

fn to_u8(c: f32) -> u8 {
    (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// src over dst with straight alpha, in sRGB as WebGL's blending works.
fn blend(dst: &mut [u8; 4], src: [f32; 3], alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let da = dst[3] as f32 / 255.0;
    let oa = a + da * (1.0 - a);
    for k in 0..3 {
        let d = dst[k] as f32 / 255.0;
        dst[k] = to_u8((src[k] * a + d * da * (1.0 - a)) / oa.max(1e-6));
    }
    dst[3] = to_u8(oa);
}

/// A face of the unit cube through a matrix: its four corners (counter-clockwise from outside) and normal.
struct Face {
    corners: [Vec3; 4],
    normal: Vec3,
}

fn faces(m: &Mat4) -> impl Iterator<Item = Face> + '_ {
    // normal, then u and v with u × v = normal
    const AXES: [([f64; 3], [f64; 3], [f64; 3]); 6] = [
        ([1., 0., 0.], [0., 1., 0.], [0., 0., 1.]),
        ([-1., 0., 0.], [0., 0., 1.], [0., 1., 0.]),
        ([0., 1., 0.], [0., 0., 1.], [1., 0., 0.]),
        ([0., -1., 0.], [1., 0., 0.], [0., 0., 1.]),
        ([0., 0., 1.], [1., 0., 0.], [0., 1., 0.]),
        ([0., 0., -1.], [0., 1., 0.], [1., 0., 0.]),
    ];
    AXES.iter().map(move |(n, u, v)| {
        let (n, u, v) = (v3(n[0], n[1], n[2]) * 0.5, v3(u[0], u[1], u[2]) * 0.5, v3(v[0], v[1], v[2]) * 0.5);
        let corners = [n - u - v, n + u - v, n + u + v, n - u + v].map(|p| m.transform_point(p));
        let normal = (corners[1] - corners[0]).cross(corners[3] - corners[0]).normalize();
        Face { corners, normal }
    })
}

fn clip_vertex(vp: &Mat4, p: Vec3, a: [f32; NA]) -> ClipVertex {
    let m = &vp.0;
    let pos = [
        (m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12]) as f32,
        (m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13]) as f32,
        (m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14]) as f32,
        (m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15]) as f32,
    ];
    ClipVertex { pos, a }
}

fn to_screen(tri: &[ClipVertex; 3], w: usize, h: usize) -> [ScreenVertex; 3] {
    tri.map(|v| {
        let iw = 1.0 / v.pos[3];
        let a = v.a.map(|x| x * iw);
        ScreenVertex {
            x: (v.pos[0] * iw + 1.0) * 0.5 * w as f32,
            y: (1.0 - v.pos[1] * iw) * 0.5 * h as f32,
            z: v.pos[2] * iw * 0.5 + 0.5,
            iw,
            a,
        }
    })
}

/// Positive when the triangle runs counter-clockwise as seen (y up), so faces toward the camera.
fn signed_area(s: &[ScreenVertex; 3]) -> f32 {
    -((s[1].x - s[0].x) * (s[2].y - s[0].y) - (s[2].x - s[0].x) * (s[1].y - s[0].y))
}

/// MeshStandardMaterial's light at a point: everything but the sun, and the sun's share (which shadows take away).
fn light(color: [f32; 3], roughness: f32, metalness: f32, n: Vec3, view: Vec3, l: &Lights) -> ([f32; 3], [f32; 3]) {
    let diffuse = color.map(|c| c * (1.0 - metalness));
    let spec_color = color.map(|c| 0.04 + (c - 0.04) * metalness);
    let rough = roughness.clamp(0.0525, 1.0);
    let alpha = rough * rough;
    let n32 = [n.x as f32, n.y as f32, n.z as f32];
    let v32 = [view.x as f32, view.y as f32, view.z as f32];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    // Hemisphere light: sky above, ground below.
    let t = 0.5 * n32[1] + 0.5;
    let mut base = [0.0f32; 3];
    for k in 0..3 {
        base[k] = diffuse[k] * (l.ground[k] + (l.sky[k] - l.ground[k]) * t);
    }
    let direct = |dir: Vec3, c: [f32; 3]| -> [f32; 3] {
        let ld = [dir.x as f32, dir.y as f32, dir.z as f32];
        let ndl = dot(n32, ld).max(0.0);
        if ndl <= 0.0 {
            return [0.0; 3];
        }
        let hv = [ld[0] + v32[0], ld[1] + v32[1], ld[2] + v32[2]];
        let hl = sqrtf(dot(hv, hv)).max(1e-6);
        let half = hv.map(|x| x / hl);
        let ndv = dot(n32, v32).clamp(0.0, 1.0);
        let ndh = dot(n32, half).clamp(0.0, 1.0);
        let ldh = dot(ld, half).clamp(0.0, 1.0);
        let fresnel = libm::exp2f((-5.55473 * ldh - 6.98316) * ldh);
        let a2 = alpha * alpha;
        let gv = ndl * sqrtf(a2 + (1.0 - a2) * ndv * ndv);
        let gl = ndv * sqrtf(a2 + (1.0 - a2) * ndl * ndl);
        let g = 0.5 / (gv + gl).max(1e-6);
        let denom = ndh * ndh * (a2 - 1.0) + 1.0;
        let d = a2 / (PI * denom * denom);
        let mut out = [0.0; 3];
        for k in 0..3 {
            let f = (1.0 - spec_color[k]) * fresnel + spec_color[k];
            out[k] = c[k] * ndl * (diffuse[k] + PI * f * g * d);
        }
        out
    };
    for (dir, c) in [(l.fill.dir, l.fill.color), (l.rim.dir, l.rim.color)] {
        let d = direct(dir, c);
        for k in 0..3 {
            base[k] += d[k];
        }
    }
    let sun = direct(l.sun.dir, l.sun.color);
    (base, sun)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawd_core::{Mode, Options};

    #[test]
    fn draws_him_in_the_middle_and_leaves_the_corners_alone() {
        let mut c = Clawd::new(Options { mode: Mode::Pet, width: 343.0, height: 223.0, seed: 1 });
        c.play_from("idle", true, 0);
        c.update(0.016);
        let mut r = Renderer::new(343, 223, Settings { background: Background::Transparent, ..Settings::default() });
        let px = r.render(&c).to_vec();
        let at = |x: usize, y: usize| &px[(y * 343 + x) * 4..][..4];
        assert_eq!(at(2, 2)[3], 0, "corner stays see-through");
        let rect = c.screen_rect().unwrap();
        let mid = at((rect.x + rect.w / 2.0) as usize, (rect.y + rect.h / 2.0) as usize);
        assert_eq!(mid[3], 255);
        assert!(mid[0] > mid[2], "clay is warm: {mid:?}");
    }

    #[test]
    fn stage_has_a_floor_and_shadows() {
        let mut c = Clawd::new(Options { mode: Mode::Stage, width: 320.0, height: 200.0, seed: 1 });
        c.play_from("idle", true, 0);
        c.update(0.016);
        let mut r = Renderer::new(320, 200, Settings::default());
        let px = r.render(&c);
        assert!(px.chunks(4).all(|p| p[3] == 255));
    }
}
