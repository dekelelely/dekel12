//! A C interface to Clawd's engine and renderer (see clawd.h). The Switch app links it as a static library built
//! for aarch64-unknown-none: there, memory comes from the C library's malloc and a panic aborts. (Builds that unwind,
//! such as tests, use std.)
#![cfg_attr(all(not(feature = "std"), panic = "abort"), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use clawd_core::moves::MOVES;
use clawd_core::{Clawd, Mode, Options};
use clawd_raster::{Background, Renderer, Settings};
use core::ffi::{c_char, c_int, CStr};

pub struct ClawdView {
    clawd: Clawd,
    renderer: Renderer,
}

#[cfg(all(not(feature = "std"), panic = "abort"))]
mod c_runtime {
    use core::alloc::{GlobalAlloc, Layout};
    use core::ffi::c_void;

    extern "C" {
        fn aligned_alloc(align: usize, size: usize) -> *mut c_void;
        fn free(p: *mut c_void);
        fn abort() -> !;
    }

    struct CAlloc;

    unsafe impl GlobalAlloc for CAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // aligned_alloc wants the size to be a multiple of the alignment
            let align = layout.align().max(16);
            let size = layout.size().div_ceil(align) * align;
            unsafe { aligned_alloc(align, size.max(align)) as *mut u8 }
        }
        unsafe fn dealloc(&self, p: *mut u8, _layout: Layout) {
            unsafe { free(p as *mut c_void) }
        }
    }

    #[global_allocator]
    static ALLOC: CAlloc = CAlloc;

    #[panic_handler]
    fn panic(_: &core::panic::PanicInfo) -> ! {
        unsafe { abort() }
    }
}

fn view<'a>(p: *mut ClawdView) -> Option<&'a mut ClawdView> {
    unsafe { p.as_mut() }
}

fn view_ref<'a>(p: *const ClawdView) -> Option<&'a ClawdView> {
    unsafe { p.as_ref() }
}

/// # Safety
/// Returns a pointer to free with `clawd_free`.
#[no_mangle]
pub extern "C" fn clawd_new(
    mode: c_int,
    width: f64,
    height: f64,
    pixel_width: c_int,
    pixel_height: c_int,
    background: c_int,
    shadow_size: c_int,
    soft_shadows: c_int,
    seed: u32,
) -> *mut ClawdView {
    let mode = if mode == 1 { Mode::Pet } else { Mode::Stage };
    let mut clawd = Clawd::new(Options { mode, width, height, seed });
    clawd.set_colors(0x2b3138, 0x4a525c, 0x4a525c, 0x4cc9db); // the apps' dark stage
    let settings = Settings {
        background: if background == 0 { Background::Transparent } else { Background::App },
        shadow_size: if shadow_size > 0 { shadow_size as usize } else { 1024 },
        soft_shadows: soft_shadows != 0,
        supersample: 1,
        effects: true,
    };
    let renderer = Renderer::new(pixel_width.max(1) as usize, pixel_height.max(1) as usize, settings);
    Box::into_raw(Box::new(ClawdView { clawd, renderer }))
}

/// # Safety
/// `p` comes from `clawd_new` and is not used again.
#[no_mangle]
pub unsafe extern "C" fn clawd_free(p: *mut ClawdView) {
    if !p.is_null() {
        drop(unsafe { Box::from_raw(p) });
    }
}

#[no_mangle]
pub extern "C" fn clawd_resize(p: *mut ClawdView, width: f64, height: f64, pixel_width: c_int, pixel_height: c_int) {
    if let Some(v) = view(p) {
        v.clawd.resize(width, height);
        v.renderer.resize(pixel_width.max(1) as usize, pixel_height.max(1) as usize);
    }
}

#[no_mangle]
pub extern "C" fn clawd_update(p: *mut ClawdView, dt: f64) -> c_int {
    view(p).map_or(0, |v| v.clawd.update(dt) as c_int)
}

#[no_mangle]
pub extern "C" fn clawd_render(p: *mut ClawdView) -> *const u8 {
    view(p).map_or(core::ptr::null(), |v| v.renderer.render(&v.clawd).as_ptr())
}

/// # Safety
/// `name` is a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn clawd_play(p: *mut ClawdView, name: *const c_char) -> c_int {
    let (Some(v), false) = (view(p), name.is_null()) else { return 0 };
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else { return 0 };
    v.clawd.play(name) as c_int
}

#[no_mangle]
pub extern "C" fn clawd_entrance(p: *mut ClawdView) {
    if let Some(v) = view(p) {
        v.clawd.entrance(false);
    }
}

/// Move names as NUL-terminated statics, for C.
const fn names() -> [&'static [u8]; 22] {
    [
        b"jump\0",
        b"look\0",
        b"celebrate\0",
        b"skip\0",
        b"spin\0",
        b"peekaboo\0",
        b"drop\0",
        b"waddle\0",
        b"peek\0",
        b"wink\0",
        b"boop\0",
        b"tap\0",
        b"sneeze\0",
        b"turn\0",
        b"coin-hop\0",
        b"back-flip\0",
        b"front-flip\0",
        b"side-flip\0",
        b"dumbbell-curl\0",
        b"laptop\0",
        b"ultracode\0",
        b"idle\0",
    ]
}
const NAMES: [&[u8]; 22] = names();

fn c_name(name: &str) -> *const c_char {
    NAMES.iter().find(|n| &n[..n.len() - 1] == name.as_bytes()).map_or(c"?".as_ptr(), |n| n.as_ptr() as *const c_char)
}

#[no_mangle]
pub extern "C" fn clawd_current(p: *const ClawdView) -> *const c_char {
    view_ref(p).map_or(core::ptr::null(), |v| c_name(v.clawd.current()))
}

#[no_mangle]
pub extern "C" fn clawd_move_count() -> c_int {
    MOVES.len() as c_int
}

#[no_mangle]
pub extern "C" fn clawd_move_name(index: c_int) -> *const c_char {
    MOVES.get(index.max(0) as usize).map_or(core::ptr::null(), |m| c_name(m.name))
}

#[no_mangle]
pub extern "C" fn clawd_tap(p: *mut ClawdView, x: f64, y: f64) -> c_int {
    let Some(v) = view(p) else { return 0 };
    if v.clawd.hits(x, y) {
        v.clawd.tap();
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn clawd_orbit(p: *mut ClawdView, dx: f64, dy: f64, tilt: c_int) {
    if let Some(v) = view(p) {
        v.clawd.orbit(dx, dy, tilt != 0);
    }
}

#[no_mangle]
pub extern "C" fn clawd_gaze(p: *mut ClawdView, x: f64, y: f64) {
    if let Some(v) = view(p) {
        v.clawd.set_gaze(Some((x, y)));
    }
}

#[no_mangle]
pub extern "C" fn clawd_look_ahead(p: *mut ClawdView) {
    if let Some(v) = view(p) {
        v.clawd.set_gaze(None);
    }
}

#[no_mangle]
pub extern "C" fn clawd_set_auto(p: *mut ClawdView, on: c_int) {
    if let Some(v) = view(p) {
        v.clawd.set_auto(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn clawd_auto(p: *const ClawdView) -> c_int {
    view_ref(p).is_some_and(|v| v.clawd.auto()) as c_int
}

#[no_mangle]
pub extern "C" fn clawd_set_ultra(p: *mut ClawdView, on: c_int) {
    if let Some(v) = view(p) {
        v.clawd.set_ultra(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn clawd_ultra(p: *const ClawdView) -> c_int {
    view_ref(p).is_some_and(|v| v.clawd.ultra()) as c_int
}

#[no_mangle]
pub extern "C" fn clawd_set_ambient(p: *mut ClawdView, on: c_int) {
    if let Some(v) = view(p) {
        v.clawd.set_ambient(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn clawd_work(p: *mut ClawdView, on: c_int) {
    if let Some(v) = view(p) {
        v.clawd.work(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn clawd_working(p: *const ClawdView) -> c_int {
    view_ref(p).is_some_and(|v| v.clawd.working()) as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_cover_every_move() {
        for m in MOVES.iter() {
            assert_ne!(c_name(m.name), c"?".as_ptr(), "{}", m.name);
        }
    }

    #[test]
    fn a_view_plays_and_draws() {
        let v = clawd_new(1, 320.0, 208.0, 320, 208, 0, 512, 1, 3);
        assert_eq!(unsafe { clawd_play(v, c"jump".as_ptr()) }, 1);
        assert_eq!(unsafe { CStr::from_ptr(clawd_current(v)) }.to_str().unwrap(), "jump");
        for _ in 0..10 {
            clawd_update(v, 1.0 / 60.0);
        }
        assert!(!clawd_render(v).is_null());
        unsafe { clawd_free(v) };
    }
}
