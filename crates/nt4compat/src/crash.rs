//! A crash report on Alpha under mrustc (no debugger or Dr. Watson log to go on otherwise):
//! when an exception reaches the unhandled-exception filter, standard error gets its code,
//! address and parameters, and from the CONTEXT the return address, stack pointer and the
//! first words on the stack. With the linker's map, those name the functions.

use core::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(n: u32) -> *mut c_void;
    fn WriteFile(h: *mut c_void, buf: *const u8, len: u32, written: *mut u32, ov: *mut c_void) -> i32;
    fn VirtualQuery(address: *const c_void, info: *mut u32, length: usize) -> usize;
    fn IsBadReadPtr(p: *const c_void, n: usize) -> i32;
}

struct Out {
    buf: [u8; 2048],
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
        let mut o = Out { buf: [0; 2048], len: 0 };
        o.s("\r\n*** exception ");
        o.hex(*(rec as *const u32) as u64, 8);
        o.s(" at ");
        o.hex(*(rec.add(12) as *const u32) as u64, 8);
        let n = *(rec.add(16) as *const u32);
        o.s(", ");
        o.hex(n as u64, 2);
        o.s(" parameters");
        for i in 0..n.min(6) {
            o.s(" ");
            o.hex(*(rec.add(20 + 4 * i as usize) as *const u32) as u64, 8);
        }
        let int = |i: usize| *(ctx.add(256 + 8 * i) as *const u64);
        // (fir, ra, t12 in full: a 32-bit address in an Alpha register must be sign-extended)
        o.s("\r\n    fir ");
        o.hex(*(ctx.add(528) as *const u64), 16);
        o.s(" ra ");
        o.hex(int(26), 16);
        o.s(" t12 ");
        o.hex(int(27), 16);
        o.s(" sp ");
        o.hex(int(30), 8);
        o.s(" a0 ");
        o.hex(int(16), 8);
        o.s(" a1 ");
        o.hex(int(17), 8);
        o.s(" v0 ");
        o.hex(int(0), 8);
        // The faulting page (MEMORY_BASIC_INFORMATION: base, allocation base, allocation
        // protection, region size, state, protection, type) and the instruction there
        let fir = *(ctx.add(528) as *const u64) as usize;
        let mut mbi = [0u32; 7];
        if VirtualQuery(fir as *const c_void, mbi.as_mut_ptr(), 28) != 0 {
            o.s("\r\n    page: base ");
            o.hex(mbi[0] as u64, 8);
            o.s(" alloc ");
            o.hex(mbi[1] as u64, 8);
            o.s(" size ");
            o.hex(mbi[3] as u64, 8);
            o.s(" state ");
            o.hex(mbi[4] as u64, 5);
            o.s(" protect ");
            o.hex(mbi[5] as u64, 3);
            o.s(" type ");
            o.hex(mbi[6] as u64, 7);
        }
        if IsBadReadPtr(fir as *const c_void, 4) == 0 {
            o.s(" word ");
            o.hex(*(fir as *const u32) as u64, 8);
        } else {
            o.s(" unreadable");
        }
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
        // A rough backtrace: the words up the stack that point into the image's code, up to
        // 60 of them in up to 256 KB (the map names their functions)
        o.s("\r\n    code addresses up the stack:");
        let mut found = 0;
        let mut i = 0usize;
        // (within the stack's region: VirtualQuery gives where it ends)
        let mut end = sp as usize;
        let mut smbi = [0u32; 7];
        if !sp.is_null() && VirtualQuery(sp as *const c_void, smbi.as_mut_ptr(), 28) != 0 {
            end = smbi[0] as usize + smbi[3] as usize;
        }
        while (sp.add(i) as usize) + 4 <= end && found < 60 && i < 65536 {
            let a = sp.add(i);
            let v = *a;
            if (0x0040_1000..0x0080_0000).contains(&v) && v & 3 == 0 {
                if found % 8 == 0 {
                    o.s("\r\n     ");
                }
                o.s(" ");
                o.hex(v as u64, 8);
                found += 1;
            }
            i += 1;
        }
        o.s("\r\n");
        let mut w = 0u32;
        WriteFile(GetStdHandle(-12i32 as u32), o.buf.as_ptr(), o.len as u32, &mut w, core::ptr::null_mut());
    }
}

// An experiment for the thread-start fault (a new thread faulting on the first instruction
// of std's thread_start, a valid PAGE_EXECUTE_READ page): std's threads start in this
// trampoline, which reports and then calls the real routine.
type StartRoutine = unsafe extern "system" fn(*mut c_void) -> u32;

type CreateThreadFn = unsafe extern "system" fn(
    *mut c_void, usize, Option<StartRoutine>, *mut c_void, u32, *mut u32,
) -> *mut c_void;

#[link(name = "kernel32")]
extern "system" {
    // (the same prototypes as random.rs's: one C prototype per name)
    fn GetProcAddress(m: crate::sys::HANDLE, name: *const u8) -> *const c_void;
    fn LoadLibraryA(name: *const u8) -> crate::sys::HANDLE;
    fn HeapAlloc(heap: *mut c_void, flags: u32, size: usize) -> *mut c_void;
    fn GetProcessHeap() -> *mut c_void;
}

fn note(what: &str, a: usize, b: usize) {
    let mut o = Out { buf: [0; 2048], len: 0 };
    o.s(what);
    o.s(" ");
    o.hex(a as u64, 8);
    o.s(" ");
    o.hex(b as u64, 8);
    o.s("\r\n");
    let mut w = 0u32;
    // SAFETY: plain calls.
    unsafe { WriteFile(GetStdHandle(-12i32 as u32), o.buf.as_ptr(), o.len as u32, &mut w, core::ptr::null_mut()) };
}

unsafe extern "system" fn trampoline(p: *mut c_void) -> u32 {
    // SAFETY: create_thread passed a block of [routine, parameter].
    unsafe {
        let b = p as *const usize;
        let (f, arg) = (*b, *b.add(1));
        note("thread trampoline: routine, parameter", f, arg);
        note("thread trampoline: routine's first word", *(f as *const u32) as usize, 0);
        let r = core::mem::transmute::<usize, StartRoutine>(f)(arg as *mut c_void);
        note("thread trampoline: routine returned", r as usize, 0);
        r
    }
}

#[export_name = "CreateThread"]
unsafe extern "system" fn create_thread(
    sa: *mut c_void, stack: usize, start: Option<StartRoutine>, param: *mut c_void, flags: u32, id: *mut u32,
) -> *mut c_void {
    // SAFETY: plain calls; the block lives as long as the thread (never freed: an experiment).
    unsafe {
        let b = HeapAlloc(GetProcessHeap(), 0, 8) as *mut usize;
        *b = start.map_or(0, |f| f as usize);
        *b.add(1) = param as usize;
        note("CreateThread: stack, flags", stack, flags as usize);
        // kernel32's own (the import library's member would bring a second CreateThread)
        let real = GetProcAddress(LoadLibraryA(b"kernel32.dll\0".as_ptr()), b"CreateThread\0".as_ptr());
        let real = core::mem::transmute::<*const c_void, CreateThreadFn>(real);
        let h = real(sa, stack, Some(trampoline), b as *mut c_void, flags, id);
        note("CreateThread: handle, routine", h as usize, *b);
        h
    }
}
