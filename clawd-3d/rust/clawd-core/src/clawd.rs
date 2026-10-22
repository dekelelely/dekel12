//! Clawd himself: plays moves frame by frame, eases his pose toward each frame, and poses the model, the way
//! clawd-engine.js's create() does. It draws nothing; `scene` lists the boxes and pictures to draw (clawd-raster
//! draws them). Random numbers come in the same order as in the engine, so the same seed gives the same sparks.
use crate::math::*;
use crate::model::*;
use crate::moves::*;
use crate::textures::*;
use alloc::vec::Vec;
use core::f64::consts::PI;
use libm::{cos, exp, sin, sqrt};

/// "stage": the floor strip, drag to turn. "pet": no floor, just his shadow, for a small floating window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Stage,
    Pet,
}

/// What happened, for the host to pass on (the engine's on... callbacks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Move(&'static str),
    Auto(bool),
    Work(bool),
    Ultra(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Seq {
    Idle,
    Move(usize),
}

impl Seq {
    fn frames(self) -> &'static [Frame] {
        match self {
            Seq::Idle => IDLE,
            Seq::Move(i) => MOVES[i].frames,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Seq::Idle => "idle",
            Seq::Move(i) => MOVES[i].name,
        }
    }
}

// The pose he eases toward each frame, value by value (the engine's state/goal objects, in the same order).
const X: usize = 0;
const Y: usize = 1;
const RY: usize = 2;
const CROUCH: usize = 3;
const ARM_L: usize = 4;
const ARM_R: usize = 5;
const LIFT_L: usize = 6;
const LIFT_R: usize = 7;
const EYE_X: usize = 8;
const EYE_Y: usize = 9;
const LID_L: usize = 10;
const LID_R: usize = 11;
const SQUASH: usize = 12;
const LAPTOP: usize = 13;
const TYPE: usize = 14;
const GUITAR: usize = 15;
const STRUM: usize = 16;
const BANG: usize = 17;
const FRET: usize = 18;
const LEAN: usize = 19;
const KICK: usize = 20;
const WINDMILL: usize = 21;
const FLIP: usize = 22;
const ROLL: usize = 23;
const DUMBBELL: usize = 24;
const CURL_L: usize = 25;
const CURL_R: usize = 26;
const WAVE: usize = 27;
const N: usize = 28;
const IS_ANGLE: [bool; N] = {
    let mut a = [false; N];
    a[RY] = true;
    a[FLIP] = true;
    a[ROLL] = true;
    a
};
const REST: [f64; N] = {
    let mut a = [0.0; N];
    a[SQUASH] = 1.0;
    a[FRET] = 0.3;
    a
};
/// Closer than this to its goal, a value has arrived (well under a pixel at any size).
const SETTLED: f64 = 0.004;
const AMBIENT_GAP: f64 = 5.0;
const TAU: f64 = 2.0 * PI;
const RIPPLE_GROW: f64 = 0.9;
const RIPPLE_HOLD: f64 = 0.35;
const RIPPLE_FADE: f64 = 1.0;

/// The engine's random numbers come from Math.random; this is a small seeded generator in its place (the same one
/// the comparison tests put in Math.random, so both draw the same).
#[derive(Clone, Debug)]
pub struct Rng(f64);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng((seed % 2147483646 + 1) as f64)
    }
    pub fn next_f64(&mut self) -> f64 {
        self.0 = (self.0 * 16807.0) % 2147483647.0;
        self.0 / 2147483647.0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Particle {
    pub live: bool,
    pub pos: Vec3,
    pub v: Vec3,
    pub rot: Vec3,
    pub spin: Vec3,
    pub scale: f64,
    pub life: f64,
    pub phase: f64,
    pub color: usize,
}

struct Pool {
    parts: Vec<Particle>,
    live: usize,
}

impl Pool {
    fn new(count: usize, colors: usize) -> Pool {
        let parts = (0..count)
            .map(|i| Particle { color: if colors > 0 { i % colors } else { 0 }, ..Default::default() })
            .collect();
        Pool { parts, live: 0 }
    }
    fn spawn(&mut self, count: usize, rng: &mut Rng, mut init: impl FnMut(&mut Particle, &mut Rng, usize)) {
        let mut made = 0;
        for p in self.parts.iter_mut() {
            if p.live {
                continue;
            }
            p.live = true;
            p.scale = 1.0;
            init(p, rng, made);
            made += 1;
            if made == count {
                break;
            }
        }
        self.live += made;
    }
    fn recount(&mut self) {
        self.live = self.parts.iter().filter(|p| p.live).count();
    }
}

/// What a drifting pixel note is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    /// A yellow music note over his head.
    Above,
    /// A grey music note beside him.
    Side,
    /// A pale blue Z (napping).
    Z,
}

/// A pixel note drifting up.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub pos: Vec3,
    pub kind: NoteKind,
    /// Which music-note shape (textures::NOTE_SHAPES); Zs use textures::Z_SHAPE.
    pub shape: usize,
    pub life: f64,
}

/// Code flooding out from him (the laptop move): time since it started, and where it started (in clip space).
#[derive(Clone, Copy, Debug)]
pub struct Ripple {
    pub t: f64,
    pub x: f64,
    pub y: f64,
}

impl Ripple {
    pub const EDGE: f64 = 46.0;
    pub const INSIDE: f64 = 0.32;
    /// How much of it is left (1 until it starts fading).
    pub fn fade(&self) -> f64 {
        1.0 - clamp((self.t - RIPPLE_GROW - RIPPLE_HOLD) / RIPPLE_FADE, 0.0, 1.0)
    }
    /// Its radius, given how far the farthest corner is from where it started (both in pixels).
    pub fn radius(&self, far: f64, edge: f64) -> f64 {
        far * libm::pow(self.t / RIPPLE_GROW, 0.85).min(1.0) + edge
    }
}

/// A box to draw: the unit cube (−½…½ on each axis) through `world`.
#[derive(Clone, Copy, Debug)]
pub struct DrawBox {
    pub world: Mat4,
    pub material: Material,
    pub cast_shadow: bool,
    pub receive_shadow: bool,
    /// Hidden below the floor line (his parts, so he can sink into it).
    pub clip: bool,
    pub double_sided: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    /// The laptop's screen (the screen texture, as it is).
    Screen,
    /// The pixel wall behind the guitar (the wall texture, see-through).
    Wall,
    /// Pet mode's floor: only the shadows that fall on it.
    ShadowCatcher,
}

/// A flat picture to draw: the unit square (−½…½ in x and y, facing +z) through `world`.
#[derive(Clone, Copy, Debug)]
pub struct DrawQuad {
    pub world: Mat4,
    pub surface: Surface,
    pub opacity: f32,
    pub clip: bool,
}

#[derive(Default)]
pub struct Scene {
    pub boxes: Vec<DrawBox>,
    pub quads: Vec<DrawQuad>,
}

/// The camera: position, view (world → camera) and projection.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub view: Mat4,
    pub projection: Mat4,
}

impl Camera {
    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }
}

/// A rectangle in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

struct View {
    theta: f64,
    phi: f64,
    t_theta: f64,
    t_phi: f64,
    r: f64,
}

pub struct Clawd {
    mode: Mode,
    rng: Rng,
    events: Vec<Event>,
    seq: Seq,
    idx: usize,
    acc: f64,
    clock: f64,
    auto: bool,
    following: bool,
    ambient: bool,
    next_ambient: f64,
    last_tap_at: f64,
    ultra: bool,
    working: bool,
    zapping: bool,
    spark_clock: f64,
    flash: f64,
    note_clock: f64,
    state: [f64; N],
    goal: [f64; N],
    eye_base: (f64, f64),
    eyes_free: bool,
    gaze: Option<(f64, f64)>,
    // the laptop's screen
    screen_no: usize,
    typed: usize,
    type_target: usize,
    pub screen: Texture,
    pub wall: Texture,
    // particles and effects
    sparks: Pool,
    confetti: Pool,
    dust: Pool,
    notes: Vec<Note>,
    note_slot: usize,
    z_slot: usize,
    ripple: Option<Ripple>,
    // looks
    materials: Materials,
    lights: Lights,
    floor_color: [f32; 3],
    dot_color: [f32; 3],
    dust_color: [f32; 3],
    code_color: u32,
    // the camera
    view: View,
    target: Vec3,
    width: f64,
    height: f64,
    camera: Camera,
    // the posed model, as update() leaves it
    root: Mat4,
    body: Mat4,
    eyes: [Mat4; 2],
    arms: [Mat4; 2],
    legs: [Mat4; 4],
    laptop_shown: f64,
    hinge: f64,
    guitar_shown: f64,
    dumbbell_shown: f64,
    wall_visible: bool,
}

/// What you pass to `Clawd::new`.
pub struct Options {
    pub mode: Mode,
    /// Size of the view in CSS pixels (or any unit the host clicks in).
    pub width: f64,
    pub height: f64,
    pub seed: u32,
}

impl Clawd {
    pub fn new(o: Options) -> Clawd {
        let materials = Materials::new();
        let mut c = Clawd {
            mode: o.mode,
            rng: Rng::new(o.seed),
            events: Vec::new(),
            seq: Seq::Idle,
            idx: 0,
            acc: 0.0,
            clock: 0.0,
            auto: false,
            following: false,
            ambient: false,
            next_ambient: 0.0,
            last_tap_at: f64::NEG_INFINITY,
            ultra: false,
            working: false,
            zapping: false,
            spark_clock: 0.0,
            flash: 0.0,
            note_clock: 0.0,
            state: REST,
            goal: REST,
            eye_base: (0.0, 0.0),
            eyes_free: true,
            gaze: None,
            screen_no: 0,
            typed: usize::MAX,
            type_target: 0,
            screen: Texture::new(SCREEN_W, SCREEN_H, false),
            wall: Texture::new(WALL_COLS * WALL_CELL, WALL_ROWS * WALL_CELL, true),
            sparks: Pool::new(48, SPARK_COLORS.len()),
            confetti: Pool::new(90, CONFETTI_COLORS.len()),
            dust: Pool::new(28, 0),
            notes: Vec::new(),
            note_slot: 0,
            z_slot: 0,
            ripple: None,
            floor_color: materials.floor.color,
            dot_color: materials.dots.color,
            dust_color: materials.dust.color,
            materials,
            lights: Lights::new(),
            code_color: 0x0e7486,
            view: View { theta: 0.0, phi: 0.0, t_theta: 0.0, t_phi: 0.0, r: 60.0 },
            target: Vec3::default(),
            width: o.width.max(1.0),
            height: o.height.max(1.0),
            camera: Camera { position: Vec3::default(), view: Mat4::IDENTITY, projection: Mat4::IDENTITY },
            root: Mat4::IDENTITY,
            body: Mat4::IDENTITY,
            eyes: [Mat4::IDENTITY; 2],
            arms: [Mat4::IDENTITY; 2],
            legs: [Mat4::IDENTITY; 4],
            laptop_shown: 0.0,
            hinge: 0.0,
            guitar_shown: 0.0,
            dumbbell_shown: 0.0,
            wall_visible: false,
        };
        c.draw_screen(0);
        c.set_mode(o.mode);
        c.pose(0.0);
        c
    }

    // ---- Settings ----

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        let f = self.framing();
        self.target = v3(f.target[0], f.target[1], f.target[2]);
        self.view.theta = f.theta;
        self.view.t_theta = f.theta;
        self.view.phi = f.phi;
        self.view.t_phi = f.phi;
        self.resize(self.width, self.height);
    }

    fn framing(&self) -> Framing {
        if self.mode == Mode::Pet {
            FRAMING_PET
        } else {
            FRAMING_STAGE
        }
    }

    /// The view's size (CSS pixels): the camera backs off until the framing fits.
    pub fn resize(&mut self, width: f64, height: f64) {
        self.width = width.max(1.0);
        self.height = height.max(1.0);
        let f = self.framing();
        let aspect = self.width / self.height;
        let tan_half = libm::tan(15f64.to_radians());
        self.view.r = (f.half_h / tan_half).max(f.half_w / (tan_half * aspect));
        self.place_camera();
    }

    pub fn size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    /// The stage's colours, as sRGB hex: the floor, its dots, the dust, and the code ripple.
    pub fn set_colors(&mut self, floor: u32, dots: u32, dust: u32, code: u32) {
        self.floor_color = lin(floor);
        self.dot_color = lin(dots);
        self.dust_color = lin(dust);
        self.code_color = code;
    }

    pub fn code_color(&self) -> u32 {
        self.code_color
    }

    pub fn lights(&self) -> &Lights {
        &self.lights
    }

    /// Takes the events since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        core::mem::take(&mut self.events)
    }

    // ---- Turning a frame into targets ----

    fn targets_for(&mut self, f: &Frame) {
        let g = &mut self.goal;
        g[X] = f.x * COL;
        g[RY] = f.angle * PI / 180.0;
        g[FLIP] = f.flip * PI / 180.0;
        g[ROLL] = f.roll * PI / 180.0;
        g[CROUCH] = if f.offset == 1 { 1.0 } else { f.squat }; // a squat bends the legs but keeps them in view
        let mut y = 0.0;
        if f.offset >= 2 {
            y = -SINK[(f.offset as usize).min(3)];
        }
        if f.offset < 0 {
            y = -(f.offset as f64) * BODY_H / 2.0;
        }
        match f.shadow {
            Shadow::Wide => y += 3.0,
            Shadow::Narrow => y += 6.0,
            Shadow::None => {}
        }
        if f.lift != 0.0 {
            y += 3.2 * f.lift;
        }
        g[Y] = y;
        g[SQUASH] = if f.offset == 1 {
            0.94
        } else if f.lift != 0.0 || f.shadow != Shadow::None {
            1.04
        } else {
            1.0
        };
        g[ARM_L] = if f.arms == Arms::Up { 0.95 } else { 0.0 };
        g[ARM_R] = if f.arms == Arms::Up || f.arms == Arms::OneUp { 0.95 } else { 0.0 };
        g[LIFT_L] = if f.feet == Feet::Right { 1.0 } else { 0.0 };
        g[LIFT_R] = if f.feet == Feet::Left { 1.0 } else { 0.0 };
        self.eye_base = match f.eyes {
            Eyes::Left => (-1.0, 0.8),
            Eyes::Right => (1.0, 0.8),
            _ => (0.0, 0.0),
        };
        self.eyes_free = f.eyes == Eyes::Open;
        let g = &mut self.goal;
        g[LID_L] = if f.eyes == Eyes::Closed { 1.0 } else { 0.0 };
        g[LID_R] = if f.eyes == Eyes::Closed || f.eyes == Eyes::Wink { 1.0 } else { 0.0 };
        g[LAPTOP] = f.laptop;
        g[TYPE] = if f.typing { 1.0 } else { 0.0 };
        g[GUITAR] = f.guitar;
        g[STRUM] = f.strum;
        g[BANG] = f.bang;
        if !f.fret.is_nan() {
            g[FRET] = f.fret;
        }
        g[LEAN] = f.lean;
        g[KICK] = f.kick;
        g[WINDMILL] = if f.windmill { 1.0 } else { 0.0 };
        g[DUMBBELL] = f.dumbbell;
        g[CURL_L] = f.curl_l;
        g[CURL_R] = f.curl_r;
        g[WAVE] = f.wave;
        self.zapping = f.zap;
        // While ultra effort is on, Clawd keeps the guitar between moves and jams quietly.
        if self.ultra && self.seq == Seq::Idle {
            g[GUITAR] = 1.0;
            g[STRUM] = 0.35;
            g[BANG] = 0.25;
            g[FRET] = 0.35;
            self.zapping = false;
        }
        self.apply_gaze();
    }

    /// Where he looks when the frame leaves his eyes open and straight ahead: a point in the view's pixels (it may
    /// lie outside the view), or None to look straight ahead.
    pub fn set_gaze(&mut self, at: Option<(f64, f64)>) {
        self.gaze = at.filter(|(x, y)| x.is_finite() && y.is_finite());
        self.apply_gaze();
    }

    fn apply_gaze(&mut self) {
        self.goal[EYE_X] = self.eye_base.0;
        self.goal[EYE_Y] = self.eye_base.1;
        let Some((gx, gy)) = self.gaze.filter(|_| self.eyes_free) else { return };
        let (w, h) = (self.width, self.height);
        let o = self.root.origin();
        let (p, _) = self.camera.view_projection().project(v3(o.x, o.y + LEG_H + BODY_H * 0.74, DEPTH / 2.0));
        let (ex, ey) = ((p.x + 1.0) / 2.0 * w, (1.0 - p.y) / 2.0 * h);
        let reach = (0.6 * w.min(h)).max(80.0); // this far away, he looks as far as his eyes go
        self.goal[EYE_X] = clamp((gx - ex) / reach, -1.0, 1.0);
        self.goal[EYE_Y] = 0.7 * clamp((ey - gy) / reach, -1.0, 1.0);
    }

    // ---- Playback ----

    fn enter_frame(&mut self) {
        let f = self.seq.frames()[self.idx];
        self.targets_for(&f);
        match f.poof {
            Poof::None => {}
            Poof::Dot => self.puff(false),
            Poof::Wave => self.puff(true),
        }
        if f.ripple {
            self.start_ripple();
            self.type_target = 0;
        }
        match f.notes {
            Notes::None => {}
            Notes::Z if self.idx.is_multiple_of(6) => self.add_note(NoteKind::Z),
            Notes::Z => {}
            n if self.idx.is_multiple_of(3) => {
                self.add_note(if n == Notes::Above { NoteKind::Above } else { NoteKind::Side })
            }
            _ => {}
        }
        if f.flash {
            self.flash = 1.0;
            self.emit_sparks(28, true);
        }
        if f.confetti > 0 {
            self.emit_confetti(f.confetti as usize);
        }
        if f.typing {
            self.type_target = (self.type_target + 4).min(SCREENS[self.screen_no].len());
        }
        if f.laptop != 0.0 {
            self.draw_screen(self.type_target);
        }
    }

    /// Plays a move by name ("idle" too); false if there is no such move.
    pub fn play(&mut self, name: &str) -> bool {
        self.play_from(name, false, 0)
    }

    /// `snap`: take the first frame's pose at once instead of easing into it. `from`: the frame to start at.
    pub fn play_from(&mut self, name: &str, snap: bool, from: usize) -> bool {
        let seq = if name == "idle" {
            Seq::Idle
        } else {
            match find(name) {
                Some(i) => Seq::Move(i),
                None => return false,
            }
        };
        self.seq = seq;
        self.idx = from.min(seq.frames().len() - 1);
        self.acc = 0.0;
        self.events.push(Event::Move(seq.name()));
        self.enter_frame();
        if snap {
            self.state = self.goal;
        }
        true
    }

    /// The move playing now ("idle" between moves).
    pub fn current(&self) -> &'static str {
        self.seq.name()
    }

    fn pick(&mut self, list: &[&'static str]) -> &'static str {
        list[(libm::floor(self.rng.next_f64() * list.len() as f64)) as usize]
    }

    fn pick_other(&mut self, list: &[&'static str]) -> &'static str {
        let now = self.seq.name();
        let others: Vec<&'static str> = list.iter().copied().filter(|n| *n != now).collect();
        self.pick(&others)
    }

    fn auto_moves(&self) -> Vec<&'static str> {
        if self.mode == Mode::Pet {
            ON_CLICK.to_vec()
        } else {
            MOVES.iter().map(|m| m.name).collect()
        }
    }

    fn schedule_ambient(&mut self) {
        self.next_ambient = self.clock + AMBIENT_GAP;
    }

    fn finished(&mut self) {
        if self.working {
            self.play_from("laptop", false, LAPTOP_OPEN);
        } else if self.auto && !self.following {
            // Auto-play: a random move straight after each one ends, never the same one twice in a row.
            let list = self.auto_moves();
            let next = self.pick_other(&list);
            self.play(next);
        } else {
            if self.seq != Seq::Idle {
                self.schedule_ambient(); // a move just ended: the next random one is due in 5 s
            }
            self.play("idle");
        }
    }

    pub fn auto(&self) -> bool {
        self.auto
    }

    pub fn set_auto(&mut self, on: bool) {
        self.auto = on;
        self.events.push(Event::Auto(on));
        if on && !self.following && !self.working && self.seq == Seq::Idle {
            let list = self.auto_moves();
            let next = self.pick_other(&list);
            self.play(next);
        }
    }

    /// Another view picks the random moves and this one only plays what it is told.
    pub fn set_following(&mut self, on: bool) {
        self.following = on;
    }

    pub fn ultra(&self) -> bool {
        self.ultra
    }

    /// Ultra effort: the guitar routine now, and the guitar kept between moves.
    pub fn set_ultra(&mut self, on: bool) {
        self.ultra = on;
        self.events.push(Event::Ultra(on));
        if on {
            self.play("ultracode");
        } else if self.seq == Seq::Idle {
            let f = self.seq.frames()[self.idx];
            self.targets_for(&f);
        }
    }

    /// Floating: a random move AMBIENT_GAP seconds after the last one ends, and taps take over.
    pub fn set_ambient(&mut self, on: bool) {
        self.ambient = on;
        self.schedule_ambient();
    }

    pub fn working(&self) -> bool {
        self.working
    }

    /// Working, for Claude Code: he opens the laptop and types, session after session, until work(false).
    pub fn work(&mut self, on: bool) {
        if on == self.working {
            return;
        }
        self.working = on;
        self.events.push(Event::Work(on));
        if on {
            if self.seq == Seq::Idle {
                self.play_from("laptop", false, LAPTOP_OPEN);
            }
        } else if self.seq.name() == "laptop" && (TYPING_FROM..=TYPING_TO).contains(&self.idx) {
            self.idx = TYPING_TO + 1; // close the laptop now
            self.acc = 0.0;
            self.enter_frame();
        }
    }

    /// Holds the current frame `ms` longer (to line up with sound that reaches the speakers late).
    pub fn hold(&mut self, ms: f64) {
        if ms > 0.0 {
            self.acc -= ms;
        }
    }

    /// A random move to start with.
    pub fn entrance(&mut self, snap: bool) {
        let name = self.pick(&ENTRANCES);
        self.play_from(name, snap, 0);
    }

    /// A tap on him: floating, every tap plays a new move at once; on the stage only while he is idle.
    pub fn tap(&mut self) {
        if self.ambient {
            self.last_tap_at = self.clock;
            self.set_auto(false);
            let next = self.pick_other(&ON_CLICK);
            self.play(next);
            return;
        }
        if self.seq != Seq::Idle {
            return;
        }
        self.set_auto(false);
        let next = self.pick(&ON_CLICK);
        self.play(next);
    }

    // ---- The laptop's screen ----

    fn draw_screen(&mut self, n: usize) {
        if n == self.typed {
            return;
        }
        self.typed = n;
        draw_screen(&mut self.screen, SCREENS[self.screen_no], n);
    }

    fn next_screen(&mut self) {
        self.screen_no = (self.screen_no + 1) % SCREENS.len();
        self.type_target = 0;
        self.typed = usize::MAX;
    }

    // ---- Particles and effects ----

    fn guitar_world(&self) -> Mat4 {
        self.body
            * Mat4::compose(
                v3(GUITAR_X, GUITAR_Y, DEPTH / 2.0 + 0.75),
                v3(0., 0., GUITAR_TILT),
                Order::Xyz,
                v3(1., 1., 1.),
            )
    }

    /// Lightning off the strings: all around (a power chord), or off to his left (the viewer's right).
    fn emit_sparks(&mut self, count: usize, burst: bool) {
        let from = self.guitar_world().origin();
        self.sparks.spawn(count, &mut self.rng, |p, r, _| {
            let a = if burst { r.next_f64() * PI * 2.0 } else { -0.4 + r.next_f64() * 1.6 };
            let sp = if burst { 10.0 + r.next_f64() * 8.0 } else { 6.0 + r.next_f64() * 6.0 };
            p.pos = from;
            p.v = v3(cos(a) * sp, sin(a) * sp + if burst { 0.0 } else { 2.0 }, (r.next_f64() - 0.3) * 3.0);
            p.life = 0.35 + r.next_f64() * 0.3;
            p.rot = v3(0.0, 0.0, r.next_f64() * PI);
        });
    }

    /// Confetti thrown up from over his head, fluttering and spinning down.
    fn emit_confetti(&mut self, count: usize) {
        let o = self.root.origin();
        self.confetti.spawn(count, &mut self.rng, |p, r, _| {
            let x = o.x + (r.next_f64() - 0.5) * 6.0;
            let z = (r.next_f64() - 0.5) * 3.0;
            p.pos = v3(x, o.y + 12.0, z);
            let vx = (r.next_f64() - 0.5) * 16.0;
            let vy = 5.0 + r.next_f64() * 6.0;
            let vz = (r.next_f64() - 0.5) * 8.0;
            p.v = v3(vx, vy, vz); // peaks within the frame
            let (sx, sy, sz) = (r.next_f64() * 10.0 - 5.0, r.next_f64() * 10.0 - 5.0, r.next_f64() * 10.0 - 5.0);
            p.spin = v3(sx, sy, sz);
            p.life = 2.0 + r.next_f64() * 0.8;
            p.phase = r.next_f64() * PI * 2.0;
            let (rx, ry, rz) = (r.next_f64() * 3.0, r.next_f64() * 3.0, r.next_f64() * 3.0);
            p.rot = v3(rx, ry, rz);
        });
    }

    /// Dust at his feet: "·" a small puff, "~" a wider one.
    fn puff(&mut self, wave: bool) {
        let spread = if wave { 7.0 } else { 4.0 };
        let x = self.root.origin().x;
        self.dust.spawn(if wave { 10 } else { 4 }, &mut self.rng, |p, r, made| {
            let side = if made % 2 == 1 { 1.0 } else { -1.0 };
            p.pos = v3(x + side * (BODY_W / 2.0 - 0.5), 0.35, (r.next_f64() - 0.5) * DEPTH);
            let vx = side * (2.0 + r.next_f64() * spread);
            let vy = 1.0 + r.next_f64() * 2.0;
            let vz = (r.next_f64() - 0.5) * 3.0;
            p.v = v3(vx, vy, vz);
            p.life = 0.6 + r.next_f64() * 0.3;
            p.rot = Vec3::default();
        });
    }

    fn add_note(&mut self, kind: NoteKind) {
        let x = self.root.origin().x;
        if kind == NoteKind::Z {
            // Zs drift up to his right, each a little higher.
            let n = (self.z_slot % 3) as f64;
            self.z_slot += 1;
            self.notes.push(Note { pos: v3(x + 6.5 + 1.6 * n, 13.0 + 1.2 * n, 1.0), kind, shape: 0, life: 1.0 });
            return;
        }
        let above = kind == NoteKind::Above;
        let pos = if above {
            let slot = self.note_slot;
            self.note_slot += 1;
            v3(x + -3.5 + 3.5 * (slot % 3) as f64, 13.0, 1.0)
        } else {
            let px = x + 9.0 + self.rng.next_f64() * 1.5;
            let py = 3.0 + self.rng.next_f64() * 4.0;
            v3(px, py, 2.0)
        };
        let shape = if self.rng.next_f64() < 0.5 { 0 } else { 1 };
        self.notes.push(Note { pos, kind, shape, life: 1.0 });
    }

    fn start_ripple(&mut self) {
        let x = self.root.origin().x;
        let (p, _) = self.camera.view_projection().project(v3(x, 6.0, 0.0));
        self.ripple = Some(Ripple { t: 0.0, x: p.x, y: p.y });
    }

    // ---- Every frame ----

    /// Moves time on by `dt` seconds (at most 0.05 is used, as the engine does). Returns whether the picture changed.
    pub fn update(&mut self, dt: f64) -> bool {
        let dt = dt.clamp(0.0, 0.05);
        self.clock += dt;
        self.acc += dt * 1000.0;
        while self.acc >= FRAME_MS {
            self.acc -= FRAME_MS;
            self.idx += 1;
            if self.working && self.seq.name() == "laptop" && self.idx > TYPING_TO {
                self.idx = TYPING_FROM; // keep typing: the next session
                self.next_screen();
            }
            if self.idx >= self.seq.frames().len() {
                self.finished();
            } else {
                self.enter_frame();
            }
        }
        // Floating: once he has idled AMBIENT_GAP seconds since his last move and the last tap, a random move.
        if self.ambient
            && !self.auto
            && !self.following
            && !self.working
            && self.seq == Seq::Idle
            && self.clock >= self.next_ambient
            && self.clock >= self.last_tap_at + AMBIENT_GAP
        {
            let next = self.pick(&ON_CLICK);
            self.play(next);
        }

        let k = 1.0 - exp(-dt / 0.045);
        let mut moving = false;
        for (i, &angle) in IS_ANGLE.iter().enumerate() {
            let d = if angle { wrap(self.goal[i] - self.state[i]) } else { self.goal[i] - self.state[i] };
            if d.abs() < SETTLED {
                self.state[i] += d; // arrived: land exactly, so he can come to rest
            } else {
                self.state[i] += d * k;
                moving = true;
            }
        }
        self.pose(dt);

        // While he keeps the guitar between moves, a note drifts up now and then.
        if self.zapping {
            self.spark_clock += dt;
            while self.spark_clock > 0.08 {
                self.spark_clock -= 0.08;
                self.emit_sparks(2, false);
            }
        }
        if self.ultra && self.seq == Seq::Idle {
            self.note_clock += dt;
            if self.note_clock > 1.1 {
                self.note_clock = 0.0;
                let kind = if self.rng.next_f64() < 0.5 { NoteKind::Above } else { NoteKind::Side };
                self.add_note(kind);
            }
        }
        self.update_particles(dt);
        self.update_fx(dt);

        let ease = (dt * 8.0).min(1.0);
        self.view.theta += (self.view.t_theta - self.view.theta) * ease;
        self.view.phi += (self.view.t_phi - self.view.phi) * ease;
        let turning =
            (self.view.t_theta - self.view.theta).abs() > 1e-4 || (self.view.t_phi - self.view.phi).abs() > 1e-4;
        if !turning {
            self.view.theta = self.view.t_theta;
            self.view.phi = self.view.t_phi;
        }
        self.place_camera();

        let s = &self.state;
        moving
            || turning
            || s[TYPE] > 0.0
            || s[STRUM] > 0.0
            || s[BANG] > 0.0
            || s[WINDMILL] > 0.0
            || s[WAVE] > 0.0
            || self.zapping
            || self.flash > 0.0
            || self.wall_visible
            || self.sparks.live > 0
            || self.confetti.live > 0
            || self.dust.live > 0
            || self.ripple.is_some()
            || !self.notes.is_empty()
    }

    /// Poses the model from the state (the engine's update, from the flip to the eyes).
    fn pose(&mut self, dt: f64) {
        let s = self.state;
        let clock = self.clock;
        // Flips turn him around his middle, not his feet: shift him back by however far the turn moves his middle.
        let middle = v3(0.0, LEG_H + BODY_H / 2.0, 0.0);
        let turned = Mat4::rotation(s[FLIP], 0.0, s[ROLL], Order::Xyz).transform_dir(middle);
        let shift = (-(turned - middle)).rotate_about(v3(0., 1., 0.), s[RY]);
        self.root = Mat4::compose(
            v3(s[X] + shift.x, s[Y] + shift.y, shift.z),
            v3(s[FLIP], s[RY], s[LEAN] + s[ROLL]),
            Order::Yxz,
            v3(1., 1., 1.),
        );
        let leg_scale = 1.0 - 0.85 * s[CROUCH];
        let dip = s[TYPE] * 0.25 * (1.0 - sin(clock * 42.0).abs()); // dips into each keystroke
        let sq = s[SQUASH];
        self.body = self.root
            * Mat4::compose(
                v3(0.0, LEG_H * leg_scale + dip, 0.0),
                v3(s[BANG] * 0.14 * sin(clock * 13.0), 0.0, 0.0), // headbanging
                Order::Xyz,
                v3(1.0 / sqrt(sq), sq, 1.0 / sqrt(sq)),
            );
        for (i, home_x) in LEG_XS.into_iter().enumerate() {
            let left = home_x < 0.0;
            let outer = home_x.abs() > 3.0;
            let stomp = s[STRUM] * 0.55 * sin(clock * 6.5 + if left { 0.0 } else { PI }).max(0.0);
            let kick = if left { 0.0 } else { s[KICK] * if outer { 1.0 } else { 0.5 } }; // a rock kick, left legs
            let lift = if left { s[LIFT_L] } else { s[LIFT_R] };
            self.legs[i] = self.root
                * Mat4::compose(
                    v3(home_x + 1.2 * kick, LEG_H * leg_scale + 0.9 * lift + stomp + 1.3 * kick, 0.0),
                    v3(0., 0., 0.),
                    Order::Xyz,
                    v3(1.0, leg_scale, 1.0),
                );
        }
        // Typing: the arms reach forward and stay level; the hands slide onto the keys, side to side, in and out.
        for i in 0..2 {
            let side = if i == 1 { 1.0 } else { -1.0 };
            let fi = i as f64;
            let press = sin(clock * 42.0 + fi * PI).max(0.0);
            let slide = sin(clock * 9.0 + fi * 2.1);
            let mut px = side * (BODY_W / 2.0 - 2.6 * s[TYPE]) + s[TYPE] * 0.8 * slide;
            let mut py = BODY_H * 0.5 - s[TYPE] * (BODY_H * 0.5 - 1.9) - s[TYPE] * 0.75 * press;
            let reach = 0.5 + 0.5 * sin(clock * 12.0 + fi * 2.4);
            let mut pz = s[TYPE] * (DEPTH / 2.0 + 0.1 - 0.9 * reach);
            let mut ry = -side * PI / 2.0 * s[TYPE];
            let mut rz = side * if i == 1 { s[ARM_R] } else { s[ARM_L] };
            if i == 1 {
                rz += s[WAVE] * 0.45 * sin(clock * 15.0); // waving his raised right arm
            }
            // Holding the guitar: the right hand (viewer's left) strums over the pickups, the left frets the neck.
            let hold = clamp(s[GUITAR] * 1.5 - 0.3, 0.0, 1.0);
            if hold > 0.0 {
                let on_guitar = |d: f64| (GUITAR_X + d * cos(GUITAR_TILT), GUITAR_Y + d * sin(GUITAR_TILT));
                let (gx, gy) = if i == 1 {
                    let wobble = if self.zapping { 0.08 * sin(clock * 9.0) } else { 0.0 };
                    on_guitar(3.0 + 4.0 * clamp(s[FRET] + wobble, 0.0, 1.0))
                } else {
                    on_guitar(-0.3)
                };
                let mut tx = gx;
                let mut ty = gy + if i == 1 { 0.0 } else { s[STRUM] * 0.9 * sin(clock * 24.0) };
                if i == 0 && s[WINDMILL] > 0.01 {
                    // Windmill: the strumming hand swings in a big circle, high over the strings and back down.
                    let a = clock * 13.0;
                    tx += s[WINDMILL] * (2.6 * cos(a) - 0.6);
                    ty += s[WINDMILL] * (3.2 + 3.2 * sin(a));
                }
                px += (tx - px) * hold;
                py += (ty - py) * hold;
                pz += (DEPTH / 2.0 - 0.3 - pz) * hold;
                ry += (-side * PI / 2.0 - ry) * hold;
                rz *= 1.0 - hold;
            }
            // Dumbbell curls: each arm swings a weight from beside his hip up to beside his head.
            let grip = clamp(s[DUMBBELL] * 1.5 - 0.3, 0.0, 1.0);
            if grip > 0.0 {
                let curl = if i == 1 { s[CURL_R] } else { s[CURL_L] };
                rz += (side * (-1.2 + 2.45 * curl) - rz) * grip;
                ry += (-side * 0.3 - ry) * grip;
            }
            self.arms[i] = self.body * Mat4::compose(v3(px, py, pz), v3(0.0, ry, rz), Order::Xyz, v3(1., 1., 1.));
        }
        self.dumbbell_shown = clamp(s[DUMBBELL] * 2.0, 0.0, 1.0);
        self.guitar_shown = clamp(s[GUITAR] * 2.0, 0.0, 1.0);
        self.flash = (self.flash - dt * 2.5).max(0.0);
        self.wall_visible = s[GUITAR] > 0.02;
        if self.wall_visible {
            draw_wall(&mut self.wall, clock, self.flash);
        }
        self.laptop_shown = clamp(s[LAPTOP] * 2.5, 0.0, 1.0);
        self.hinge = clamp((s[LAPTOP] - 0.2) / 0.8, 0.0, 1.0) * 105f64.to_radians();
        for (i, home_x) in EYE_XS.into_iter().enumerate() {
            let lid = if i == 0 { s[LID_L] } else { s[LID_R] };
            self.eyes[i] = self.body
                * Mat4::compose(
                    v3(home_x + s[EYE_X], BODY_H * 0.74 + s[EYE_Y] - 0.45 * lid, DEPTH / 2.0 + 0.25),
                    v3(0., 0., 0.),
                    Order::Xyz,
                    v3(1.0, 1.0 - 0.82 * lid, 1.0),
                );
        }
    }

    fn update_particles(&mut self, dt: f64) {
        if self.sparks.live > 0 {
            for p in self.sparks.parts.iter_mut().filter(|p| p.live) {
                p.life -= dt * 1.4;
                if p.life <= 0.0 {
                    p.live = false;
                    continue;
                }
                p.v = p.v * (1.0 - 3.0 * dt);
                p.pos = p.pos + p.v * dt;
                p.rot.z += dt * 9.0;
                p.scale = (p.life * 3.0).min(1.0);
            }
            self.sparks.recount();
        }
        if self.confetti.live > 0 {
            let clock = self.clock;
            for p in self.confetti.parts.iter_mut().filter(|p| p.live) {
                p.life -= dt;
                if p.life <= 0.0 {
                    p.live = false;
                    continue;
                }
                p.v.y = (p.v.y - 16.0 * dt).max(-3.2); // falls fast at first, then flutters down
                p.v.x *= 1.0 - 1.2 * dt;
                p.v.z *= 1.0 - 1.2 * dt;
                p.pos = p.pos + p.v * dt;
                p.pos.x += sin(clock * 5.0 + p.phase) * 1.6 * dt;
                if p.pos.y <= 0.05 {
                    // settles on the floor
                    p.pos.y = 0.05;
                    p.v = Vec3::default();
                    p.spin = Vec3::default();
                    p.rot.x = -PI / 2.0;
                }
                p.rot = p.rot + p.spin * dt;
                p.scale = (p.life * 2.0).min(1.0);
            }
            self.confetti.recount();
        }
        if self.dust.live > 0 {
            for p in self.dust.parts.iter_mut().filter(|p| p.live) {
                p.life -= dt * 1.7;
                if p.life <= 0.0 {
                    p.live = false;
                    continue;
                }
                p.v.y -= 5.0 * dt;
                p.v = p.v * (1.0 - 2.4 * dt);
                p.pos = p.pos + p.v * dt;
                p.pos.y = p.pos.y.max(0.3);
                p.scale = p.life;
            }
            self.dust.recount();
        }
    }

    fn update_fx(&mut self, dt: f64) {
        if let Some(r) = self.ripple.as_mut() {
            r.t += dt;
            if r.fade() <= 0.0 {
                self.ripple = None;
            }
        }
        self.notes.retain_mut(|n| {
            n.life -= dt * 1.3;
            n.pos.y += dt * 1.5;
            n.life > 0.0
        });
    }

    // ---- Camera, picking ----

    fn place_camera(&mut self) {
        let (t, v) = (self.target, &self.view);
        let sp = sin(v.phi);
        let position = v3(t.x + v.r * sp * sin(v.theta), t.y + v.r * cos(v.phi), t.z + v.r * sp * cos(v.theta));
        let world = Mat4::look_at(position, t, v3(0., 1., 0.));
        self.camera = Camera {
            position,
            view: world.invert().unwrap_or(Mat4::IDENTITY),
            projection: Mat4::perspective(30.0, self.width / self.height, 0.5, 400.0),
        };
    }

    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Turns the stage (drag): `dx`, `dy` in pixels; `tilt` too only with a mouse, as on the web page.
    pub fn orbit(&mut self, dx: f64, dy: f64, tilt: bool) {
        self.view.t_theta -= dx * 0.008;
        if tilt {
            self.view.t_phi = clamp(self.view.t_phi - dy * 0.006, 0.85, 1.48);
        }
    }

    /// His body's boxes (torso, eyes, arms, legs) as world matrices of the unit cube: what taps can hit.
    pub fn body_boxes(&self) -> [Mat4; 9] {
        let sized = |m: Mat4, w: f64, h: f64, d: f64, at: Vec3| m * Mat4::translation(at) * Mat4::scaling(v3(w, h, d));
        [
            sized(self.body, BODY_W, BODY_H, DEPTH, v3(0.0, BODY_H / 2.0, 0.0)),
            sized(self.eyes[0], EYE, EYE, 0.6, Vec3::default()),
            sized(self.eyes[1], EYE, EYE, 0.6, Vec3::default()),
            sized(self.arms[0], ARM_LEN, 2.6, 2.2, v3(-ARM_LEN / 2.0, 0.0, 0.0)),
            sized(self.arms[1], ARM_LEN, 2.6, 2.2, v3(ARM_LEN / 2.0, 0.0, 0.0)),
            sized(self.legs[0], 1.3, LEG_H, 2.2, v3(0.0, -LEG_H / 2.0, 0.0)),
            sized(self.legs[1], 1.3, LEG_H, 2.2, v3(0.0, -LEG_H / 2.0, 0.0)),
            sized(self.legs[2], 1.3, LEG_H, 2.2, v3(0.0, -LEG_H / 2.0, 0.0)),
            sized(self.legs[3], 1.3, LEG_H, 2.2, v3(0.0, -LEG_H / 2.0, 0.0)),
        ]
    }

    /// Whether a point of the view (pixels) is on him: a ray from the camera through it hits one of his boxes above
    /// the floor line.
    pub fn hits(&self, x: f64, y: f64) -> bool {
        let ndc = v3(x / self.width * 2.0 - 1.0, -(y / self.height * 2.0 - 1.0), 0.5);
        let Some(inv) = self.camera.view_projection().invert() else { return false };
        let (far, _) = inv.project(ndc);
        let origin = self.camera.position;
        let dir = (far - origin).normalize();
        self.body_boxes().iter().any(|m| {
            let Some(local) = m.invert() else { return false };
            let (o, d) = (local.transform_point(origin), local.transform_dir(dir));
            let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
            for (oa, da) in [(o.x, d.x), (o.y, d.y), (o.z, d.z)] {
                if da.abs() < 1e-12 {
                    if !(-0.5..=0.5).contains(&oa) {
                        return false;
                    }
                    continue;
                }
                let (a, b) = ((-0.5 - oa) / da, (0.5 - oa) / da);
                t0 = t0.max(a.min(b));
                t1 = t1.min(a.max(b));
            }
            t0 <= t1 && t0 > 0.0 && (origin + dir * t0).y >= 0.0
        })
    }

    /// Where each box of his body is on screen (pixels), clipped to the view: tighter than `screen_rect` around his
    /// outline.
    pub fn screen_rects(&self) -> Vec<Rect> {
        let vp = self.camera.view_projection();
        let (w, h) = (self.width, self.height);
        self.body_boxes()
            .iter()
            .filter_map(|m| {
                let (mut x0, mut y0, mut x1, mut y1) =
                    (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
                for c in 0..8 {
                    let corner = v3(
                        if c & 1 != 0 { 0.5 } else { -0.5 },
                        if c & 2 != 0 { 0.5 } else { -0.5 },
                        if c & 4 != 0 { 0.5 } else { -0.5 },
                    );
                    let (p, _) = vp.project(m.transform_point(corner));
                    let (x, y) = ((p.x + 1.0) / 2.0 * w, (1.0 - p.y) / 2.0 * h);
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
                let (x0, x1, y0, y1) = (clamp(x0, 0.0, w), clamp(x1, 0.0, w), clamp(y0, 0.0, h), clamp(y1, 0.0, h));
                (x1 - x0 >= 1.0 && y1 - y0 >= 1.0).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
            })
            .collect()
    }

    /// The box around his whole body on screen (pixels).
    pub fn screen_rect(&self) -> Option<Rect> {
        let rects = self.screen_rects();
        let first = *rects.first()?;
        Some(rects.iter().fold(first, |a, r| {
            let (x0, y0) = (a.x.min(r.x), a.y.min(r.y));
            let (x1, y1) = ((a.x + a.w).max(r.x + r.w), (a.y + a.h).max(r.y + r.h));
            Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
        }))
    }

    // ---- What to draw ----

    pub fn ripple(&self) -> Option<Ripple> {
        self.ripple
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// Everything to draw now, into `out` (cleared first).
    pub fn scene(&self, out: &mut Scene) {
        out.boxes.clear();
        out.quads.clear();
        let m = &self.materials;
        let one = v3(1., 1., 1.);
        let mut solid = |world: Mat4, size: [f64; 3], at: Vec3, material: Material, cast: bool| {
            out.boxes.push(DrawBox {
                world: world * Mat4::translation(at) * Mat4::scaling(v3(size[0], size[1], size[2])),
                material,
                cast_shadow: cast,
                receive_shadow: true,
                clip: true,
                double_sided: false,
            });
        };
        // Clawd
        solid(self.body, [BODY_W, BODY_H, DEPTH], v3(0.0, BODY_H / 2.0, 0.0), m.clay, true);
        for e in &self.eyes {
            solid(*e, [EYE, EYE, 0.6], Vec3::default(), m.eye, true);
        }
        for (i, a) in self.arms.iter().enumerate() {
            let side = if i == 1 { 1.0 } else { -1.0 };
            solid(*a, [ARM_LEN, 2.6, 2.2], v3(side * ARM_LEN / 2.0, 0.0, 0.0), m.clay, true);
            if self.dumbbell_shown > 0.01 {
                let d = *a
                    * Mat4::compose(v3(side * (ARM_LEN + 0.1), 0.0, 0.0), Vec3::default(), Order::Xyz, one)
                    * Mat4::scaling(one * self.dumbbell_shown.max(0.001));
                solid(d, [0.7, 0.7, 5.4], Vec3::default(), m.bar, true);
                for z in [-2.05, 2.05f64] {
                    solid(d, [2.5, 2.5, 0.9], v3(0.0, 0.0, z), m.iron, true);
                    solid(d, [1.5, 1.5, 0.35], v3(0.0, 0.0, z + z.signum() * 0.6), m.bar, true);
                }
            }
        }
        for l in &self.legs {
            solid(*l, [1.3, LEG_H, 2.2], v3(0.0, -LEG_H / 2.0, 0.0), m.leg, true);
        }
        // The laptop, in front of him
        if self.laptop_shown > 0.01 {
            let laptop = self.root
                * Mat4::translation(v3(0.0, 0.0, DEPTH / 2.0 + 0.15 + LT_D / 2.0))
                * Mat4::scaling(one * self.laptop_shown.max(0.001));
            solid(laptop, [LT_W, LT_BASE, LT_D], v3(0.0, LT_BASE / 2.0, 0.0), m.laptop, true);
            let keys = DrawBox {
                world: laptop
                    * Mat4::translation(v3(0.0, LT_BASE + 0.02, LT_D * 0.1))
                    * Mat4::scaling(v3(LT_W - 1.4, 0.04, LT_D * 0.5)),
                material: m.keys,
                cast_shadow: false,
                receive_shadow: false,
                clip: true,
                double_sided: false,
            };
            out.boxes.push(keys);
            let hinge = laptop * Mat4::compose(v3(0.0, LT_BASE, LT_D / 2.0), v3(self.hinge, 0., 0.), Order::Xyz, one);
            out.boxes.push(DrawBox {
                world: hinge
                    * Mat4::translation(v3(0.0, LT_LID / 2.0, -LT_D / 2.0))
                    * Mat4::scaling(v3(LT_W, LT_LID, LT_D)),
                material: m.laptop,
                cast_shadow: true,
                receive_shadow: true,
                clip: true,
                double_sided: false,
            });
            out.quads.push(DrawQuad {
                world: hinge
                    * Mat4::compose(v3(0.0, -0.01, -LT_D / 2.0), v3(PI / 2.0, 0.0, PI), Order::Xyz, one)
                    * Mat4::scaling(v3(LT_W - 0.8, LT_D - 0.6, 1.0)),
                surface: Surface::Screen,
                opacity: 1.0,
                clip: true,
            });
        }
        // The guitar
        if self.guitar_shown > 0.01 {
            let g = self.guitar_world() * Mat4::scaling(one * self.guitar_shown.max(0.001));
            for (size, mat, at) in guitar_parts() {
                let material = match mat {
                    GuitarMat::Dark => m.dark,
                    GuitarMat::Lavender => m.lavender,
                    GuitarMat::Board => m.board,
                    GuitarMat::String => m.string,
                };
                out.boxes.push(DrawBox {
                    world: g * Mat4::translation(at) * Mat4::scaling(v3(size[0], size[1], size[2])),
                    material,
                    cast_shadow: true,
                    receive_shadow: true,
                    clip: true,
                    double_sided: false,
                });
            }
        }
        // Particles
        let basic = |c: u32| Material { color: lin(c), shading: Shading::Basic };
        for (pool, size, kind) in [
            (&self.sparks, [0.22, 0.75, 0.22], 0),
            (&self.confetti, [0.6, 0.6, 0.06], 1),
            (&self.dust, [0.6, 0.6, 0.6], 2),
        ] {
            for p in pool.parts.iter().filter(|p| p.live) {
                let material = match kind {
                    0 => basic(SPARK_COLORS[p.color]),
                    1 => basic(CONFETTI_COLORS[p.color]),
                    _ => Material { color: self.dust_color, shading: m.dust.shading },
                };
                out.boxes.push(DrawBox {
                    world: Mat4::compose(p.pos, p.rot, Order::Xyz, one * p.scale)
                        * Mat4::scaling(v3(size[0], size[1], size[2])),
                    material,
                    cast_shadow: false,
                    receive_shadow: false,
                    clip: false,
                    double_sided: kind == 1,
                });
            }
        }
        // The stage, or just his shadow
        if self.mode == Mode::Stage {
            out.boxes.push(DrawBox {
                world: Mat4::translation(v3(0.0, -0.4, 0.0)) * Mat4::scaling(v3(90.0, 0.8, 18.0)),
                material: Material { color: self.floor_color, shading: m.floor.shading },
                cast_shadow: false,
                receive_shadow: true,
                clip: false,
                double_sided: false,
            });
            for x in dot_xs() {
                out.boxes.push(DrawBox {
                    world: Mat4::translation(v3(x, 0.03, 7.6)) * Mat4::scaling(v3(0.45, 0.06, 0.45)),
                    material: Material { color: self.dot_color, shading: m.dots.shading },
                    cast_shadow: false,
                    receive_shadow: true,
                    clip: false,
                    double_sided: false,
                });
            }
        } else {
            out.quads.push(DrawQuad {
                world: Mat4::compose(Vec3::default(), v3(-PI / 2.0, 0.0, 0.0), Order::Xyz, one)
                    * Mat4::scaling(v3(60.0, 30.0, 1.0)),
                surface: Surface::ShadowCatcher,
                opacity: 0.3,
                clip: false,
            });
        }
        // The pixel wall, see-through, drawn after everything solid
        if self.wall_visible {
            let x = self.root.origin().x;
            out.quads.push(DrawQuad {
                world: Mat4::translation(v3(x, 9.0, -5.0)) * Mat4::scaling(v3(36.0, 24.0, 1.0)),
                surface: Surface::Wall,
                opacity: clamp(self.state[GUITAR] * 1.2, 0.0, 1.0) as f32,
                clip: true,
            });
        }
    }
}

/// An angle brought into −π…π.
fn wrap(a: f64) -> f64 {
    a - TAU * js_round(a / TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clawd(mode: Mode) -> Clawd {
        Clawd::new(Options { mode, width: 640.0, height: 400.0, seed: 7 })
    }

    fn run(c: &mut Clawd, seconds: f64) {
        for _ in 0..(seconds * 60.0) as usize {
            c.update(1.0 / 60.0);
        }
    }

    #[test]
    fn every_move_plays_through_and_ends_idle() {
        for m in MOVES.iter() {
            let mut c = clawd(Mode::Stage);
            assert!(c.play(m.name));
            run(&mut c, m.frames.len() as f64 * 0.06 + 0.2);
            assert_eq!(c.current(), "idle", "{}", m.name);
        }
    }

    #[test]
    fn comes_to_rest_between_moves() {
        let mut c = clawd(Mode::Pet);
        c.play_from("idle", true, 0);
        run(&mut c, 0.3);
        // Frames 1–12 of idle are all the same: after easing in, nothing changes.
        assert!(!c.update(1.0 / 60.0));
    }

    #[test]
    fn work_loops_the_typing_until_stopped() {
        let mut c = clawd(Mode::Stage);
        c.work(true);
        assert_eq!(c.current(), "laptop");
        run(&mut c, 10.0);
        assert_eq!(c.current(), "laptop");
        c.work(false);
        run(&mut c, 1.0);
        assert_eq!(c.current(), "idle");
        let events = c.take_events();
        assert!(events.contains(&Event::Work(true)) && events.contains(&Event::Work(false)));
    }

    #[test]
    fn floating_he_plays_on_his_own() {
        let mut c = clawd(Mode::Pet);
        c.set_ambient(true);
        run(&mut c, 5.5);
        assert_ne!(c.current(), "idle");
    }

    #[test]
    fn celebrate_throws_confetti_and_jump_puffs_dust() {
        let mut c = clawd(Mode::Stage);
        c.play("celebrate");
        run(&mut c, 0.35);
        assert!(c.confetti.live > 0 && c.dust.live > 0);
        let mut scene = Scene::default();
        c.scene(&mut scene);
        assert!(scene.boxes.len() > 9 + 55 + 1);
    }

    #[test]
    fn taps_on_him_hit_and_beside_him_miss() {
        let mut c = clawd(Mode::Pet);
        c.play_from("idle", true, 0);
        run(&mut c, 0.2);
        let r = c.screen_rect().unwrap();
        assert!(c.hits(r.x + r.w / 2.0, r.y + r.h / 2.0));
        assert!(!c.hits(2.0, 2.0));
        assert!(c.screen_rects().len() == 9);
    }

    #[test]
    fn gaze_moves_his_eyes_toward_the_point() {
        let mut c = clawd(Mode::Pet);
        c.play_from("idle", true, 0);
        c.set_gaze(Some((5000.0, 200.0)));
        assert!(c.goal[EYE_X] > 0.9);
        c.set_gaze(None);
        assert_eq!(c.goal[EYE_X], 0.0);
    }

    #[test]
    fn rng_matches_the_comparison_generator() {
        // seed = seed * 16807 % 2147483647, as the JS test harness stubs Math.random
        let mut r = Rng::new(10);
        assert_eq!(r.next_f64(), (11.0 * 16807.0) / 2147483647.0);
    }
}
