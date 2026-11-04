//! C-ABI exports of the spike painter for wasm32-unknown-unknown (Task 09), no wasm-bindgen.
//! Protocol (lengths in bytes, pointers into linear memory, wasm32 usize = i32):
//! 1. `alloc(len)` → ptr; the host copies its input there and frees it with `dealloc(ptr, len)`.
//! 2. `init(font, font_len, png, png_len)`: parse the font and decode the image PNG, once per instance.
//! 3. `load_scene(json, len)`: parse a renderer Scene (the spike subset); kept until the next load.
//! 4. `render(scale)`: paint the loaded scene at that DPR with the export text options (bbox painter,
//!    sequential). The premultiplied RGBA pixmap stays here until the next render:
//!    `pixels_ptr()`, `pixels_len()`, `width()`, `height()`.
//!
//! Calls return 0 or a negative code; panics abort (trap) with `panic = "abort"`.
use rust_spike::font::Shaper;
use rust_spike::mode::PaintMode;
use rust_spike::paint::Painter;
use rust_spike::scene::Scene;
use rust_spike::text::{TextPainter, EXPORT_OPTS};
use std::cell::RefCell;
use tiny_skia::Pixmap;

struct State {
    painter: Painter<'static>,
    scene: Option<Scene>,
    out: Option<Pixmap>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    std::slice::from_raw_parts(ptr, len)
}

fn with<R>(f: impl FnOnce(&mut State) -> R, missing: R) -> R {
    STATE.with_borrow_mut(|s| s.as_mut().map_or(missing, f))
}

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let ptr = v.as_mut_ptr();
    std::mem::forget(v);
    ptr
}

/// # Safety
/// `ptr` and `len` must come from one `alloc` call.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len));
}

/// # Safety
/// Both ranges must be readable linear memory.
#[no_mangle]
pub unsafe extern "C" fn init(font: *const u8, font_len: usize, png: *const u8, png_len: usize) -> i32 {
    let Ok(image) = Pixmap::decode_png(bytes(png, png_len)) else { return -1 };
    // The painter borrows the shaper for the life of the instance; a second `init` leaks the first.
    let shaper: &'static Shaper = Box::leak(Box::new(Shaper::from_bytes(bytes(font, font_len).to_vec())));
    let painter = Painter { shaper, text: TextPainter::new(shaper), image, opts: EXPORT_OPTS, mode: PaintMode::Bbox, prof: Default::default() };
    STATE.set(Some(State { painter, scene: None, out: None }));
    0
}

/// # Safety
/// The range must be readable linear memory.
#[no_mangle]
pub unsafe extern "C" fn load_scene(json: *const u8, len: usize) -> i32 {
    let Ok(scene) = serde_json::from_slice::<Scene>(bytes(json, len)) else { return -2 };
    with(|s| { s.scene = Some(scene); 0 }, -1)
}

#[no_mangle]
pub extern "C" fn render(scale: f32) -> i32 {
    with(
        |s| match &s.scene {
            Some(scene) => {
                s.out = None;
                s.out = Some(s.painter.render(scene, scale));
                0
            }
            None => -2,
        },
        -1,
    )
}

fn out<R>(f: impl FnOnce(&Pixmap) -> R, missing: R) -> R {
    STATE.with_borrow(|s| s.as_ref().and_then(|s| s.out.as_ref()).map_or(missing, f))
}

#[no_mangle]
pub extern "C" fn pixels_ptr() -> *const u8 {
    out(|p| p.data().as_ptr(), std::ptr::null())
}

#[no_mangle]
pub extern "C" fn pixels_len() -> usize {
    out(|p| p.data().len(), 0)
}

#[no_mangle]
pub extern "C" fn width() -> u32 {
    out(|p| p.width(), 0)
}

#[no_mangle]
pub extern "C" fn height() -> u32 {
    out(|p| p.height(), 0)
}
