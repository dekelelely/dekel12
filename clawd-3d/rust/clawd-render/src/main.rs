//! Pictures and GIFs of Clawd's moves, drawn by the Rust engine (no browser):
//!
//!   clawd-render celebrate --at 1.2 --out celebrate.png
//!   clawd-render ultracode --gif --out ultracode.gif --size 480x300 --fps 25
//!   clawd-render idle --mode pet --transparent --out pet.png
//!
//! Options: --mode stage|pet, --size WxH (CSS pixels), --scale N (pixels per CSS pixel), --ss N (supersampling),
//! --seed N, --at SECONDS (a picture of that moment), --gif (the whole move), --fps N, --transparent, --hard-shadows,
//! --shadow-size N, --bench (time every frame of the move instead of writing a file).
use clawd_core::{Clawd, Mode, Options};
use clawd_raster::{Background, Renderer, Settings};
use std::fs::File;
use std::io::BufWriter;
use std::process::exit;

struct Args {
    name: String,
    mode: Mode,
    width: usize,
    height: usize,
    scale: usize,
    ss: usize,
    seed: u32,
    at: f64,
    gif: bool,
    fps: f64,
    transparent: bool,
    soft: bool,
    shadow_size: usize,
    bench: bool,
    out: String,
}

fn parse() -> Args {
    let mut a = Args {
        name: String::new(),
        mode: Mode::Stage,
        width: 640,
        height: 400,
        scale: 1,
        ss: 2,
        seed: 10,
        at: 0.0,
        gif: false,
        fps: 25.0,
        transparent: false,
        soft: true,
        shadow_size: 0,
        bench: false,
        out: String::new(),
    };
    let mut it = std::env::args().skip(1);
    let usage = || -> ! {
        eprintln!("usage: clawd-render <move|idle> [--at S | --gif] [--mode stage|pet] [--size WxH] [--scale N] [--ss N] [--seed N] [--fps N] [--transparent] [--hard-shadows] --out FILE");
        exit(2)
    };
    while let Some(arg) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| usage());
        match arg.as_str() {
            "--mode" => a.mode = if value() == "pet" { Mode::Pet } else { Mode::Stage },
            "--size" => {
                let v = value();
                let (w, h) = v.split_once('x').unwrap_or_else(|| usage());
                a.width = w.parse().unwrap_or_else(|_| usage());
                a.height = h.parse().unwrap_or_else(|_| usage());
            }
            "--scale" => a.scale = value().parse().unwrap_or_else(|_| usage()),
            "--ss" => a.ss = value().parse().unwrap_or_else(|_| usage()),
            "--seed" => a.seed = value().parse().unwrap_or_else(|_| usage()),
            "--at" => a.at = value().parse().unwrap_or_else(|_| usage()),
            "--fps" => a.fps = value().parse().unwrap_or_else(|_| usage()),
            "--out" => a.out = value(),
            "--gif" => a.gif = true,
            "--transparent" => a.transparent = true,
            "--hard-shadows" => a.soft = false,
            "--shadow-size" => a.shadow_size = value().parse().unwrap_or_else(|_| usage()),
            "--bench" => a.bench = true,
            _ if a.name.is_empty() && !arg.starts_with('-') => a.name = arg,
            _ => usage(),
        }
    }
    if a.name.is_empty() || (a.out.is_empty() && !a.bench) {
        usage();
    }
    a
}

fn main() {
    let a = parse();
    let mut clawd = Clawd::new(Options { mode: a.mode, width: a.width as f64, height: a.height as f64, seed: a.seed });
    // The apps' dark stage.
    clawd.set_colors(0x2b3138, 0x4a525c, 0x4a525c, 0x4cc9db);
    if !clawd.play_from(&a.name, true, 0) {
        eprintln!("no move called {:?}", a.name);
        exit(1);
    }
    let (w, h) = (a.width * a.scale, a.height * a.scale);
    let background = if a.transparent { Background::Transparent } else { Background::App };
    let shadow_size = if a.shadow_size > 0 {
        a.shadow_size
    } else if a.mode == Mode::Pet {
        1024
    } else {
        2048
    };
    let mut renderer = Renderer::new(
        w,
        h,
        Settings { background, shadow_size, soft_shadows: a.soft, supersample: a.ss, effects: true },
    );
    let step = 1.0 / 60.0;
    clawd.update(0.0); // the engine's first frame, as on the page
    if a.bench {
        // Every frame of the move at 60 frames a second: how long updating and drawing take.
        let (mut times, mut t) = (Vec::new(), 0.0);
        while t < 30.0 && !(clawd.current() == "idle" && t > 0.5) {
            let start = std::time::Instant::now();
            clawd.update(step);
            renderer.render(&clawd);
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            t += step;
        }
        times.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        println!(
            "{} {}x{} ss{}: {} frames, mean {:.2} ms, median {:.2} ms, 95th {:.2} ms, max {:.2} ms",
            a.name,
            w,
            h,
            a.ss,
            times.len(),
            mean,
            times[times.len() / 2],
            times[times.len() * 95 / 100],
            times[times.len() - 1]
        );
        return;
    }
    if !a.gif {
        let mut t = 0.0;
        while t + step / 2.0 < a.at {
            clawd.update(step);
            t += step;
        }
        write_png(&a.out, w, h, renderer.render(&clawd));
        println!("{} at {:.2}s: {}", a.name, a.at, a.out);
        return;
    }
    // The whole move, and a little of the idle after it.
    let file = File::create(&a.out).unwrap_or_else(|e| {
        eprintln!("{}: {e}", a.out);
        exit(1)
    });
    let mut encoder = gif::Encoder::new(BufWriter::new(file), w as u16, h as u16, &[]).expect("gif");
    encoder.set_repeat(gif::Repeat::Infinite).expect("gif");
    let per_frame = 1.0 / a.fps;
    let (mut t, mut next, mut frames, mut idle_for) = (0.0, 0.0, 0, 0.0);
    while idle_for < 0.4 && t < 30.0 {
        if t + 1e-9 >= next {
            let mut px = renderer.render(&clawd).to_vec();
            let mut frame = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut px, 10);
            frame.delay = (per_frame * 100.0).round() as u16;
            encoder.write_frame(&frame).expect("gif");
            frames += 1;
            next += per_frame;
        }
        clawd.update(step);
        t += step;
        if clawd.current() == "idle" && a.name != "idle" {
            idle_for += step;
        }
    }
    println!("{}: {frames} frames, {:.1}s: {}", a.name, t, a.out);
}

fn write_png(path: &str, w: usize, h: usize, rgba: &[u8]) {
    let file = File::create(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        exit(1)
    });
    let mut enc = png::Encoder::new(BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(rgba)).expect("png");
}
