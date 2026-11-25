//! The parts of examples/stdtest that `ntrun` (tenox7/ntsdk's emulator, which runs Alpha
//! console programs without a VM) can run: std's threads, synchronization, thread-locals,
//! files, environment, time, formatting and float maths. Left out: processes and pipes,
//! sockets, directory listings, renames and the random-number checks, which call functions
//! ntrun doesn't have (PLAN-mips-alpha-ppc.md lists them). Each check prints "ok" or
//! "FAILED: why"; the last line is "stdsmoke: ok" when all passed, and the exit code is the
//! number of failures.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, Once, OnceLock, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type R = Result<(), String>;

fn check(failures: &mut u32, name: &str, f: impl FnOnce() -> R) {
    match f() {
        Ok(()) => println!("  {name:<28} ok"),
        Err(e) => {
            println!("  {name:<28} FAILED: {e}");
            *failures += 1;
        }
    }
}

fn ensure(cond: bool, what: impl FnOnce() -> String) -> R {
    if cond { Ok(()) } else { Err(what()) }
}

fn threads_and_mutex() -> R {
    let total = Arc::new(Mutex::new(0u64));
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            let total = Arc::clone(&total);
            thread::spawn(move || {
                for i in 0..1000 {
                    *total.lock().unwrap() += t * 1000 + i;
                }
                t
            })
        })
        .collect();
    let ids: u64 = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let v = *total.lock().unwrap();
    ensure(ids == 6 && v == 7_998_000, || format!("ids {ids}, total {v}"))
}

fn condvar_and_rwlock() -> R {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let p2 = Arc::clone(&pair);
    let h = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        *p2.0.lock().unwrap() = true;
        p2.1.notify_one();
    });
    let mut ready = pair.0.lock().unwrap();
    while !*ready {
        ready = pair.1.wait(ready).unwrap();
    }
    drop(ready);
    h.join().unwrap();
    let rw = RwLock::new(5);
    {
        let a = rw.read().unwrap();
        let b = rw.read().unwrap();
        ensure(*a + *b == 10, || "two readers".into())?;
    }
    *rw.write().unwrap() += 1;
    ensure(*rw.read().unwrap() == 6, || "writer".into())
}

fn channels_once_park() -> R {
    // The senders finish before the receiving starts: a receiver waiting on a slot being
    // written yields with SwitchToThread, which ntrun doesn't have.
    let (tx, rx) = mpsc::channel();
    let senders: Vec<_> = (0..3)
        .map(|t| {
            let tx = tx.clone();
            thread::spawn(move || {
                for i in 0..10 {
                    tx.send(t * 100 + i).unwrap();
                }
            })
        })
        .collect();
    drop(tx);
    for s in senders {
        s.join().map_err(|_| "sender".to_string())?;
    }
    let mut got: Vec<i32> = rx.iter().collect();
    got.sort();
    ensure(got.len() == 30 && got[0] == 0 && got[29] == 209, || format!("{} values", got.len()))?;
    static ONCE: Once = Once::new();
    static CELL: OnceLock<u32> = OnceLock::new();
    let n = AtomicUsize::new(0);
    for _ in 0..3 {
        ONCE.call_once(|| {
            n.fetch_add(1, Ordering::SeqCst);
        });
    }
    ensure(n.load(Ordering::SeqCst) == 1 && *CELL.get_or_init(|| 7) == 7, || "Once".into())?;
    let main = thread::current();
    let h = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        main.unpark();
    });
    thread::park_timeout(Duration::from_secs(5));
    h.join().map_err(|_| "unpark thread".to_string())
}

fn thread_locals() -> R {
    thread_local! {
        static COUNT: Cell<u32> = const { Cell::new(0) };
    }
    COUNT.with(|c| c.set(1));
    let other = thread::spawn(|| COUNT.with(|c| c.get())).join().unwrap();
    ensure(other == 0 && COUNT.with(|c| c.get()) == 1, || format!("other thread saw {other}"))
}

fn files() -> R {
    let path = "stdsmoke.tmp";
    let mut f = fs::File::create(path).map_err(|e| format!("create: {e}"))?;
    f.write_all(b"hello, ").and_then(|_| f.write_all(b"Alpha")).map_err(|e| format!("write: {e}"))?;
    drop(f);
    let mut f = fs::OpenOptions::new().read(true).write(true).open(path).map_err(|e| format!("open: {e}"))?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| format!("read: {e}"))?;
    f.seek(SeekFrom::Start(7)).map_err(|e| format!("seek: {e}"))?;
    f.write_all(b"NT").map_err(|e| format!("write: {e}"))?;
    let len = f.metadata().map_err(|e| format!("metadata: {e}"))?.len();
    drop(f);
    let back = fs::read_to_string(path).map_err(|e| format!("read back: {e}"))?;
    fs::remove_file(path).map_err(|e| format!("remove: {e}"))?;
    ensure(s == "hello, Alpha" && back == "hello, NTpha" && len == 12, || format!("{s:?} {back:?} {len}"))?;
    ensure(fs::metadata(path).is_err(), || "still there".into())
}

fn environment() -> R {
    // SAFETY: no other thread reads the environment now.
    unsafe { std::env::set_var("STDSMOKE_VAR", "value") };
    let v = std::env::var("STDSMOKE_VAR").map_err(|e| format!("var: {e}"))?;
    let args = std::env::args().count();
    let cwd = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    ensure(v == "value" && args >= 1 && cwd.is_absolute(), || format!("{v:?}, {args} args, {cwd:?}"))
}

fn time() -> R {
    let t0 = Instant::now();
    thread::sleep(Duration::from_millis(120));
    let dt = t0.elapsed();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| format!("{e}"))?;
    ensure(dt >= Duration::from_millis(100) && dt < Duration::from_secs(5), || format!("slept {dt:?}"))?;
    ensure(now.as_secs() > 1_600_000_000, || format!("now {now:?}"))
}

fn formatting_and_collections() -> R {
    let mut m = BTreeMap::new();
    for (i, w) in "the quick brown fox jumps over the lazy dog".split(' ').enumerate() {
        m.entry(w).or_insert_with(Vec::new).push(i);
    }
    let s = format!("{:?} {:>6.2} {:#x} {:e}", m.get("the"), 3.14159, 255u8, 1234.5f64);
    let mut v: Vec<u64> = (0..100).map(|i| (i * 7919) % 101).collect();
    v.sort_unstable();
    ensure(s == "Some([0, 6])   3.14 0xff 1.2345e3" && v[0] == 0 && v[99] == 100, || s)
}

fn float_maths() -> R {
    let x = 2.0f64;
    let checks = [
        (x.sqrt(), 1.4142135623730951),
        (x.powi(10), 1024.0),
        (x.powf(0.5), 1.4142135623730951),
        ((1.0f64).exp(), std::f64::consts::E),
        (std::f64::consts::E.ln(), 1.0),
        ((0.5f64).sin(), 0.479425538604203),
        ((1.0f64).atan2(1.0), std::f64::consts::FRAC_PI_4),
        ((7.5f64) % 2.0, 1.5),
        ((-7.5f64).rem_euclid(2.0), 0.5),
        ((2.5f64).round(), 3.0),
        ((-2.5f64).trunc(), -2.0),
    ];
    for (i, (got, want)) in checks.iter().enumerate() {
        if (got - want).abs() > 1e-12 {
            return Err(format!("#{i}: {got} != {want}"));
        }
    }
    let f = 3.5f32;
    ensure((f * f).sqrt() == 3.5 && (f64::NAN as u32) == 0 && (1e30f64 as i32) == i32::MAX, || "f32/casts".into())
}

fn main() {
    println!("std on Windows NT {}", std::env::consts::ARCH);
    let mut failures = 0;
    check(&mut failures, "threads, Mutex", threads_and_mutex);
    check(&mut failures, "Condvar, RwLock", condvar_and_rwlock);
    check(&mut failures, "channels, Once, park", channels_once_park);
    check(&mut failures, "thread_local", thread_locals);
    check(&mut failures, "files", files);
    check(&mut failures, "environment, args", environment);
    check(&mut failures, "time, sleep", time);
    check(&mut failures, "formatting, BTreeMap, sort", formatting_and_collections);
    check(&mut failures, "float maths", float_maths);
    if failures == 0 {
        println!("stdsmoke: ok");
    } else {
        println!("stdsmoke: {failures} FAILED");
    }
    std::process::exit(failures as i32);
}
