//! A crash report on Alpha under mrustc (no debugger or Dr. Watson log to go on otherwise):
//! when an exception reaches the unhandled-exception filter, standard error gets its code,
//! address and parameters, and from the CONTEXT the return address, stack pointer and the
//! first words on the stack. With the linker's map, those name the functions.

use core::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(n: u32) -> *mut c_void;
    fn WriteFile(h: *mut c_void, buf: *const u8, len: u32, written: *mut u32, ov: *mut c_void) -> i32;
}

struct Out {
    buf: [u8; 1024],
    len: usize,
}

impl Out {
    fn s(&mut self, s: &str) {
        for &b in s.as_bytes() {
            if self.len < self.buf.len() {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
    }
    fn hex(&mut self, v: u64, digits: u32) {
        let mut i = digits;
        while i > 0 {
            i -= 1;
            let d = ((v >> (i * 4)) & 15) as u8;
            let c = if d < 10 { b'0' + d } else { b'a' + d - 10 };
            if self.len < self.buf.len() {
                self.buf[self.len] = c;
                self.len += 1;
            }
        }
    }
}

/// `pointers` is the filter's EXCEPTION_POINTERS.
pub unsafe fn report(pointers: *mut c_void) {
    // SAFETY: the system passes valid EXCEPTION_POINTERS; the CONTEXT is Alpha's (winnt.h):
    // 32 floating registers, then the integer ones (Ra is #26, Sp #30), Fpcr, SoftFpcr, Fir.
    unsafe {
        let p = pointers as *const *const u8;
        let rec = *p;
        let ctx = *p.add(1);
        let mut o = Out { buf: [0; 1024], len: 0 };
        o.s("\r\n*** exception ");
        o.hex(*(rec as *const u32) as u64, 8);
        o.s(" at ");
        o.hex(*(rec.add(12) as *const u32) as u64, 8);
        let n = *(rec.add(16) as *const u32);
        o.s(", parameters");
        for i in 0..n.min(4) {
            o.s(" ");
            o.hex(*(rec.add(20 + 4 * i as usize) as *const u32) as u64, 8);
        }
        let int = |i: usize| *(ctx.add(256 + 8 * i) as *const u64);
        o.s("\r\n    fir ");
        o.hex(*(ctx.add(528) as *const u64), 8);
        o.s(" ra ");
        o.hex(int(26), 8);
        o.s(" sp ");
        o.hex(int(30), 8);
        o.s(" a0 ");
        o.hex(int(16), 8);
        o.s(" a1 ");
        o.hex(int(17), 8);
        o.s(" v0 ");
        o.hex(int(0), 8);
        o.s("\r\n    stack:");
        let sp = int(30) as usize as *const u32;
        if !sp.is_null() {
            for i in 0..48 {
                if i % 8 == 0 {
                    o.s("\r\n     ");
                }
                o.s(" ");
                o.hex(*sp.add(i) as u64, 8);
            }
        }
        o.s("\r\n");
        let mut w = 0u32;
        WriteFile(GetStdHandle(-12i32 as u32), o.buf.as_ptr(), o.len as u32, &mut w, core::ptr::null_mut());
    }
}
