//! A GUI program for PINT: a window with GDI drawing, standard controls (a button and an
//! edit box) and a common control (a progress bar). It checks itself, then saves a picture
//! of its window and closes:
//!
//!   - the window class and window are created, the controls too;
//!   - text set in the edit box, non-ASCII included, reads back the same;
//!   - the progress bar keeps its position;
//!   - WM_PAINT arrives, and the colours it painted read back with GetPixel;
//!   - a timer fires (the message loop runs);
//!   - the window's client area is copied with BitBlt and saved as GUI.BMP.
//!
//! A GUI program has no console: the results go to GUI.LOG in the current directory, ending
//! with "gui: ok". The exit code is the number of failed checks.
#![no_std]
#![no_main]
#![windows_subsystem = "windows"]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::{self, Write};
use core::ptr::{null, null_mut};

#[global_allocator]
static HEAP: nt4rt::heap::NtHeap = nt4rt::heap::NtHeap;

nt4rt::entry!(main);

mod w {
    //! The few Win32 declarations this program needs.
    #![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]
    use core::ffi::c_void;

    pub type HANDLE = *mut c_void;
    pub type HWND = HANDLE;
    pub type HDC = HANDLE;
    pub type HGDIOBJ = HANDLE;
    pub type WNDPROC = unsafe extern "system" fn(HWND, u32, usize, isize) -> isize;

    #[repr(C)]
    pub struct WNDCLASSW {
        pub style: u32,
        pub lpfnWndProc: WNDPROC,
        pub cbClsExtra: i32,
        pub cbWndExtra: i32,
        pub hInstance: HANDLE,
        pub hIcon: HANDLE,
        pub hCursor: HANDLE,
        pub hbrBackground: HANDLE,
        pub lpszMenuName: *const u16,
        pub lpszClassName: *const u16,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    pub struct RECT {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    #[repr(C)]
    pub struct MSG {
        pub hwnd: HWND,
        pub message: u32,
        pub wParam: usize,
        pub lParam: isize,
        pub time: u32,
        pub pt: [i32; 2],
    }

    #[repr(C)]
    pub struct PAINTSTRUCT {
        pub hdc: HDC,
        pub fErase: i32,
        pub rcPaint: RECT,
        pub fRestore: i32,
        pub fIncUpdate: i32,
        pub rgbReserved: [u8; 32],
    }

    #[repr(C)]
    pub struct BITMAPINFOHEADER {
        pub biSize: u32,
        pub biWidth: i32,
        pub biHeight: i32,
        pub biPlanes: u16,
        pub biBitCount: u16,
        pub biCompression: u32,
        pub biSizeImage: u32,
        pub biXPelsPerMeter: i32,
        pub biYPelsPerMeter: i32,
        pub biClrUsed: u32,
        pub biClrImportant: u32,
    }

    pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
    pub const WS_VISIBLE: u32 = 0x1000_0000;
    pub const WS_CHILD: u32 = 0x4000_0000;
    pub const WS_BORDER: u32 = 0x0080_0000;
    pub const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;
    pub const WM_DESTROY: u32 = 0x0002;
    pub const WM_PAINT: u32 = 0x000F;
    pub const WM_TIMER: u32 = 0x0113;
    pub const WM_USER: u32 = 0x0400;
    pub const PBM_SETPOS: u32 = WM_USER + 2;
    pub const SW_SHOW: i32 = 5;
    pub const IDC_ARROW: usize = 32512;
    pub const COLOR_WINDOW: usize = 5;
    pub const DT_CENTER: u32 = 0x1;
    pub const DT_VCENTER: u32 = 0x4;
    pub const DT_SINGLELINE: u32 = 0x20;
    pub const TRANSPARENT: i32 = 1;
    pub const SRCCOPY: u32 = 0x00CC_0020;
    pub const DIB_RGB_COLORS: u32 = 0;
    pub const GENERIC_WRITE: u32 = 0x4000_0000;
    pub const CREATE_ALWAYS: u32 = 2;
    pub const INVALID_HANDLE_VALUE: HANDLE = -1isize as HANDLE;

    #[link(name = "user32")]
    extern "system" {
        pub fn RegisterClassW(wc: *const WNDCLASSW) -> u16;
        pub fn CreateWindowExW(
            ex: u32, class: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32,
            h: i32, parent: HWND, menu: HANDLE, inst: HANDLE, param: *mut c_void,
        ) -> HWND;
        pub fn DefWindowProcW(h: HWND, m: u32, wp: usize, lp: isize) -> isize;
        pub fn GetMessageW(msg: *mut MSG, h: HWND, min: u32, max: u32) -> i32;
        pub fn TranslateMessage(msg: *const MSG) -> i32;
        pub fn DispatchMessageW(msg: *const MSG) -> isize;
        pub fn PostQuitMessage(code: i32);
        pub fn ShowWindow(h: HWND, cmd: i32) -> i32;
        pub fn UpdateWindow(h: HWND) -> i32;
        pub fn DestroyWindow(h: HWND) -> i32;
        pub fn BeginPaint(h: HWND, ps: *mut PAINTSTRUCT) -> HDC;
        pub fn EndPaint(h: HWND, ps: *const PAINTSTRUCT) -> i32;
        pub fn GetDC(h: HWND) -> HDC;
        pub fn ReleaseDC(h: HWND, dc: HDC) -> i32;
        pub fn GetClientRect(h: HWND, r: *mut RECT) -> i32;
        pub fn FillRect(dc: HDC, r: *const RECT, brush: HANDLE) -> i32;
        pub fn DrawTextW(dc: HDC, s: *const u16, n: i32, r: *mut RECT, fmt: u32) -> i32;
        pub fn LoadCursorW(inst: HANDLE, name: usize) -> HANDLE;
        pub fn SetTimer(h: HWND, id: usize, ms: u32, f: *const c_void) -> usize;
        pub fn KillTimer(h: HWND, id: usize) -> i32;
        pub fn SetWindowTextW(h: HWND, s: *const u16) -> i32;
        pub fn GetWindowTextW(h: HWND, s: *mut u16, n: i32) -> i32;
        pub fn SendMessageW(h: HWND, m: u32, wp: usize, lp: isize) -> isize;
    }

    #[link(name = "gdi32")]
    extern "system" {
        pub fn CreateSolidBrush(rgb: u32) -> HANDLE;
        pub fn CreatePen(style: i32, width: i32, rgb: u32) -> HANDLE;
        pub fn DeleteObject(o: HGDIOBJ) -> i32;
        pub fn SelectObject(dc: HDC, o: HGDIOBJ) -> HGDIOBJ;
        pub fn Ellipse(dc: HDC, l: i32, t: i32, r: i32, b: i32) -> i32;
        pub fn MoveToEx(dc: HDC, x: i32, y: i32, old: *mut [i32; 2]) -> i32;
        pub fn LineTo(dc: HDC, x: i32, y: i32) -> i32;
        pub fn SetBkMode(dc: HDC, mode: i32) -> i32;
        pub fn SetTextColor(dc: HDC, rgb: u32) -> u32;
        pub fn GetPixel(dc: HDC, x: i32, y: i32) -> u32;
        pub fn CreateCompatibleDC(dc: HDC) -> HDC;
        pub fn CreateCompatibleBitmap(dc: HDC, w: i32, h: i32) -> HANDLE;
        pub fn BitBlt(
            dst: HDC, x: i32, y: i32, w: i32, h: i32, src: HDC, sx: i32, sy: i32, rop: u32,
        ) -> i32;
        pub fn GetDIBits(
            dc: HDC, bm: HANDLE, start: u32, lines: u32, bits: *mut c_void,
            info: *mut BITMAPINFOHEADER, usage: u32,
        ) -> i32;
        pub fn DeleteDC(dc: HDC) -> i32;
    }

    #[link(name = "comctl32")]
    extern "system" {
        pub fn InitCommonControls();
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetModuleHandleW(name: *const u16) -> HANDLE;
        pub fn CreateFileW(
            name: *const u16, access: u32, share: u32, sa: *mut c_void, disp: u32,
            flags: u32, template: HANDLE,
        ) -> HANDLE;
        pub fn WriteFile(h: HANDLE, b: *const u8, n: u32, done: *mut u32, ov: *mut c_void) -> i32;
        pub fn CloseHandle(h: HANDLE) -> i32;
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

const RED: u32 = rgb(255, 0, 0);
const BLUE: u32 = rgb(0, 0, 255);
const GREEN: u32 = rgb(0, 128, 0);

/// The red square and the blue disc of WM_PAINT, in client coordinates.
const RED_RECT: w::RECT = w::RECT { left: 20, top: 20, right: 120, bottom: 120 };
const DISC: w::RECT = w::RECT { left: 140, top: 20, right: 240, bottom: 120 };
const EDIT_TEXT: &str = "Rust on NT 4.0 – Grüße";

/// A NUL-terminated UTF-16 copy of `s`.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}

/// A file opened for writing; `write!` goes to it.
struct File(w::HANDLE);

impl File {
    fn create(name: &str) -> Option<File> {
        let n = wide(name);
        // SAFETY: n is NUL-terminated; the other arguments are plain values or null.
        let h = unsafe {
            w::CreateFileW(n.as_ptr(), w::GENERIC_WRITE, 0, null_mut(), w::CREATE_ALWAYS, 0, null_mut())
        };
        (h != w::INVALID_HANDLE_VALUE).then_some(File(h))
    }

    fn write_all(&mut self, b: &[u8]) -> bool {
        let mut done = 0u32;
        // SAFETY: b is valid for its length; the handle is open.
        let ok = unsafe { w::WriteFile(self.0, b.as_ptr(), b.len() as u32, &mut done, null_mut()) };
        ok != 0 && done as usize == b.len()
    }
}

impl Write for File {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (i, line) in s.split('\n').enumerate() {
            if i > 0 && !self.write_all(b"\r\n") {
                return Err(fmt::Error);
            }
            if !self.write_all(line.as_bytes()) {
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // SAFETY: the handle is open and closed only here.
        unsafe { w::CloseHandle(self.0) };
    }
}

/// What the window procedure and main share. The program has one thread.
struct State {
    log: Option<File>,
    failures: u32,
    painted: bool,
    edit: w::HWND,
    progress: w::HWND,
}

static mut STATE: State =
    State { log: None, failures: 0, painted: false, edit: null_mut(), progress: null_mut() };

fn state() -> &'static mut State {
    // SAFETY: one thread; the window procedure runs only inside calls made by main's
    // message loop, and no reference is kept across those calls.
    unsafe { &mut *core::ptr::addr_of_mut!(STATE) }
}

fn check(name: &str, ok: bool) {
    let s = state();
    if let Some(log) = s.log.as_mut() {
        let _ = writeln!(log, "  {:<28} {}", name, if ok { "ok" } else { "FAILED" });
    }
    if !ok {
        s.failures += 1;
    }
}

macro_rules! log {
    ($($a:tt)*) => {
        if let Some(log) = state().log.as_mut() {
            let _ = writeln!(log, $($a)*);
        }
    };
}

fn paint(dc: w::HDC, client: &w::RECT) {
    // SAFETY: dc is the window's paint DC; the objects are created, selected back out and
    // deleted here.
    unsafe {
        let red = w::CreateSolidBrush(RED);
        w::FillRect(dc, &RED_RECT, red);
        w::DeleteObject(red);

        let blue = w::CreateSolidBrush(BLUE);
        let old_brush = w::SelectObject(dc, blue);
        w::Ellipse(dc, DISC.left, DISC.top, DISC.right, DISC.bottom);
        w::SelectObject(dc, old_brush);
        w::DeleteObject(blue);

        let pen = w::CreatePen(0, 3, GREEN);
        let old_pen = w::SelectObject(dc, pen);
        w::MoveToEx(dc, 260, 20, null_mut());
        w::LineTo(dc, 360, 120);
        w::MoveToEx(dc, 360, 20, null_mut());
        w::LineTo(dc, 260, 120);
        w::SelectObject(dc, old_pen);
        w::DeleteObject(pen);

        let text = wide("Hello from Rust on Windows NT 4.0");
        let mut r = w::RECT { top: 130, bottom: 160, ..*client };
        w::SetBkMode(dc, w::TRANSPARENT);
        w::SetTextColor(dc, rgb(0, 0, 0));
        w::DrawTextW(dc, text.as_ptr(), -1, &mut r, w::DT_CENTER | w::DT_VCENTER | w::DT_SINGLELINE);
    }
}

/// Copy the client area into a 24-bit bitmap and write it as a BMP file.
fn save_bmp(hwnd: w::HWND, name: &str) -> bool {
    let mut r = w::RECT::default();
    // SAFETY: hwnd is our window; every GDI object made here is released here.
    unsafe {
        w::GetClientRect(hwnd, &mut r);
        let (wd, ht) = (r.right - r.left, r.bottom - r.top);
        if wd <= 0 || ht <= 0 {
            return false;
        }
        let dc = w::GetDC(hwnd);
        let mem = w::CreateCompatibleDC(dc);
        let bm = w::CreateCompatibleBitmap(dc, wd, ht);
        let old = w::SelectObject(mem, bm);
        let copied = w::BitBlt(mem, 0, 0, wd, ht, dc, 0, 0, w::SRCCOPY) != 0;
        w::SelectObject(mem, old);

        let stride = ((wd * 3 + 3) & !3) as usize;
        let mut bits = alloc::vec![0u8; stride * ht as usize];
        let mut info = w::BITMAPINFOHEADER {
            biSize: core::mem::size_of::<w::BITMAPINFOHEADER>() as u32,
            biWidth: wd,
            biHeight: ht, // bottom-up, as BMP files store it
            biPlanes: 1,
            biBitCount: 24,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let lines = w::GetDIBits(mem, bm, 0, ht as u32, bits.as_mut_ptr().cast(), &mut info, w::DIB_RGB_COLORS);
        w::DeleteObject(bm);
        w::DeleteDC(mem);
        w::ReleaseDC(hwnd, dc);
        if !copied || lines != ht {
            return false;
        }

        let Some(mut f) = File::create(name) else { return false };
        let size = 14 + 40 + bits.len() as u32;
        let mut head = Vec::with_capacity(54);
        head.extend_from_slice(b"BM");
        head.extend_from_slice(&size.to_le_bytes());
        head.extend_from_slice(&0u32.to_le_bytes());
        head.extend_from_slice(&54u32.to_le_bytes());
        for v in [40u32, wd as u32, ht as u32] {
            head.extend_from_slice(&v.to_le_bytes());
        }
        head.extend_from_slice(&1u16.to_le_bytes());
        head.extend_from_slice(&24u16.to_le_bytes());
        head.extend_from_slice(&[0; 24]);
        f.write_all(&head) && f.write_all(&bits)
    }
}

/// The checks on the window as painted, from the timer.
fn check_window(hwnd: w::HWND) {
    let s = state();
    check("WM_PAINT received", s.painted);

    // SAFETY: hwnd is our window; the DC is released here.
    let (in_square, in_disc, outside) = unsafe {
        let dc = w::GetDC(hwnd);
        let p = (
            w::GetPixel(dc, 70, 70),
            w::GetPixel(dc, 190, 70),
            w::GetPixel(dc, 15, 15),
        );
        w::ReleaseDC(hwnd, dc);
        p
    };
    check("red square (GetPixel)", in_square == RED);
    check("blue disc (GetPixel)", in_disc == BLUE);
    check("background (GetPixel)", outside != RED && outside != BLUE);
    if in_square != RED || in_disc != BLUE {
        log!("    pixels {:06x} {:06x} {:06x}", in_square, in_disc, outside);
    }

    let mut buf = [0u16; 64];
    // SAFETY: buf is valid for its length.
    let n = unsafe { w::GetWindowTextW(s.edit, buf.as_mut_ptr(), buf.len() as i32) };
    let back: Vec<u16> = buf[..n.max(0) as usize].to_vec();
    let want: Vec<u16> = EDIT_TEXT.encode_utf16().collect();
    check("edit box text (UTF-16)", back == want);

    // SAFETY: a message to our progress bar.
    let prev = unsafe { w::SendMessageW(s.progress, w::PBM_SETPOS, 80, 0) };
    check("progress bar position", prev == 60);

    check("window saved as GUI.BMP", save_bmp(hwnd, "GUI.BMP"));
}

unsafe extern "system" fn wndproc(hwnd: w::HWND, msg: u32, wp: usize, lp: isize) -> isize {
    // SAFETY (whole body): Win32 calls on our own window, with valid arguments.
    unsafe {
        match msg {
            w::WM_PAINT => {
                let mut ps: w::PAINTSTRUCT = core::mem::zeroed();
                let dc = w::BeginPaint(hwnd, &mut ps);
                let mut client = w::RECT::default();
                w::GetClientRect(hwnd, &mut client);
                paint(dc, &client);
                w::EndPaint(hwnd, &ps);
                state().painted = true;
                0
            }
            w::WM_TIMER => {
                w::KillTimer(hwnd, 1);
                check("timer fired", true);
                check_window(hwnd);
                w::DestroyWindow(hwnd);
                0
            }
            w::WM_DESTROY => {
                w::PostQuitMessage(0);
                0
            }
            _ => w::DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

fn child(class: &str, text: &str, style: u32, r: (i32, i32, i32, i32), parent: w::HWND, id: usize) -> w::HWND {
    let (c, t) = (wide(class), wide(text));
    // SAFETY: the strings are NUL-terminated; parent is our window.
    unsafe {
        w::CreateWindowExW(
            0, c.as_ptr(), t.as_ptr(), w::WS_CHILD | w::WS_VISIBLE | style, r.0, r.1, r.2, r.3,
            parent, id as w::HANDLE, w::GetModuleHandleW(null()), null_mut(),
        )
    }
}

fn main() -> u32 {
    let s = state();
    s.log = File::create("GUI.LOG");
    log!("Rust GUI on Windows NT 4.0");

    // SAFETY: Win32 calls with valid arguments; the strings outlive the calls.
    unsafe {
        w::InitCommonControls();
        let inst = w::GetModuleHandleW(null());
        let class = wide("RustNt4Window");
        let wc = w::WNDCLASSW {
            style: 0,
            lpfnWndProc: wndproc,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: null_mut(),
            hCursor: w::LoadCursorW(null_mut(), w::IDC_ARROW),
            hbrBackground: (w::COLOR_WINDOW + 1) as w::HANDLE,
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
        };
        let atom = w::RegisterClassW(&wc);
        check("RegisterClassW", atom != 0);

        let title = wide("Rust on Windows NT 4.0");
        let hwnd = w::CreateWindowExW(
            0, class.as_ptr(), title.as_ptr(), w::WS_OVERLAPPEDWINDOW | w::WS_VISIBLE,
            w::CW_USEDEFAULT, w::CW_USEDEFAULT, 400, 320, null_mut(), null_mut(), inst, null_mut(),
        );
        check("CreateWindowExW", !hwnd.is_null());
        if hwnd.is_null() {
            log!("gui: {} FAILED", s.failures);
            return s.failures;
        }

        let button = child("BUTTON", "OK", 0, (20, 170, 100, 28), hwnd, 10);
        check("button control", !button.is_null());
        s.edit = child("EDIT", "", w::WS_BORDER, (140, 170, 220, 24), hwnd, 11);
        check("edit control", !s.edit.is_null());
        let t = wide(EDIT_TEXT);
        w::SetWindowTextW(s.edit, t.as_ptr());
        s.progress = child("msctls_progress32", "", 0, (20, 215, 340, 20), hwnd, 12);
        check("progress bar (comctl32)", !s.progress.is_null());
        w::SendMessageW(s.progress, w::PBM_SETPOS, 60, 0);

        w::ShowWindow(hwnd, w::SW_SHOW);
        w::UpdateWindow(hwnd);
        check("SetTimer", w::SetTimer(hwnd, 1, 1500, null()) != 0);

        let mut msg: w::MSG = core::mem::zeroed();
        while w::GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            w::TranslateMessage(&msg);
            w::DispatchMessageW(&msg);
        }
    }

    if s.failures == 0 {
        log!("gui: ok");
    } else {
        log!("gui: {} FAILED", s.failures);
    }
    s.failures
}
