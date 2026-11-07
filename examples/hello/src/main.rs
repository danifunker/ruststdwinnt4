//! The first program for PINT: prints a greeting and exercises what the runtime provides
//! (the heap, formatting, 64-bit division through NTDLL's helpers, and the conversions
//! between 64-bit integers and floating point). The last line is "hello: ok" when every
//! check passed; the exit code is the number of failed checks.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::hint::black_box;

use nt4rt::println;

#[global_allocator]
static HEAP: nt4rt::heap::NtHeap = nt4rt::heap::NtHeap;

nt4rt::entry!(main);

fn check(failures: &mut u32, name: &str, ok: bool) {
    println!("  {:<24} {}", name, if ok { "ok" } else { "FAILED" });
    if !ok {
        *failures += 1;
    }
}

fn main() -> u32 {
    // SAFETY: GetVersion takes no arguments.
    let v = unsafe { nt4rt::sys::GetVersion() };
    println!(
        "Hello from Rust on Windows NT {}.{} (build {})",
        v & 0xff,
        (v >> 8) & 0xff,
        (v >> 16) & 0x7fff
    );

    let mut failures = 0;

    let mut v: Vec<u64> = Vec::new();
    for i in 0..1000u64 {
        v.push(i * i);
    }
    check(&mut failures, "heap (Vec of 1000)", v.iter().sum::<u64>() == 332_833_500);

    #[repr(align(64))]
    struct Wide([u8; 64]);
    let b = alloc::boxed::Box::new(Wide([7; 64]));
    check(&mut failures, "heap (64-byte aligned)", (&*b as *const Wide as usize) % 64 == 0 && b.0[63] == 7);

    let s: String = alloc::format!("{}-{:x}-{:.3}", 42, 0xbeefu32, 3.14159f64);
    check(&mut failures, "format", s == "42-beef-3.142");

    let n = black_box(0x1234_5678_9abc_def0u64);
    let d = black_box(0x1_0000_0001u64);
    let (q, r) = (n / d, n % d);
    check(&mut failures, "u64 division", q == 0x1234_5678 && r == 0x8888_8878);
    if q != 0x1234_5678 || r != 0x8888_8878 {
        println!("    got {:#x} rem {:#x}", q, r);
    }
    let a = black_box(-1_000_000_007i64);
    check(&mut failures, "i64 division", a / black_box(10) == -100_000_000 && a % black_box(10) == -7);
    let u = black_box(1_000_003u32);
    check(&mut failures, "u32 division", u / black_box(7) == 142_857 && u % black_box(7) == 4);

    let big = black_box(u64::MAX);
    check(&mut failures, "u64 -> f64", (big as f64) == 18_446_744_073_709_551_615.0);
    check(&mut failures, "i64 -> f32", (black_box(-(1i64 << 40)) as f32) == -1_099_511_627_776.0);
    check(&mut failures, "f64 -> u64", (black_box(1e19f64) as u64) == 10_000_000_000_000_000_000);
    check(&mut failures, "f32 -> i64", (black_box(-123456.75f32) as i64) == -123456);

    if failures == 0 {
        println!("hello: ok");
    } else {
        println!("hello: {} FAILED", failures);
    }
    failures
}
