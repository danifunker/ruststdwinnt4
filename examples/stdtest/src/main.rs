//! std on Windows NT 4.0: one check per area of std a program like rusty-backup uses.
//! Each prints "ok" or "FAILED: why"; the last line is "stdtest: ok" when all passed, and
//! the exit code is the number of failures.

use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::Command;
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

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

fn at<E: std::fmt::Display>(what: &'static str) -> impl FnOnce(E) -> String {
    move |err| format!("{what}: {err}")
}

fn threads_and_mutex() -> R {
    let counter = Arc::new(Mutex::new(0u64));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let c = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..2000 {
                    *c.lock().unwrap() += 1;
                }
            })
        })
        .collect();
    for h in handles {
        h.join().map_err(|_| "a thread panicked".to_string())?;
    }
    let n = *counter.lock().unwrap();
    ensure(n == 16000, || format!("counter {n}, want 16000"))
}

fn condvar_and_rwlock() -> R {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let p2 = Arc::clone(&pair);
    let t = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        *p2.0.lock().unwrap() = true;
        p2.1.notify_all();
    });
    let mut ready = pair.0.lock().unwrap();
    while !*ready {
        ready = pair.1.wait(ready).unwrap();
    }
    drop(ready);
    t.join().unwrap();

    let (lock, cv) = (Mutex::new(()), Condvar::new());
    let start = Instant::now();
    let (_g, timeout) = cv.wait_timeout(lock.lock().unwrap(), Duration::from_millis(100)).unwrap();
    let waited = start.elapsed();
    ensure(timeout.timed_out() && waited >= Duration::from_millis(90), || format!("wait_timeout after {waited:?}"))?;

    let rw = Arc::new(RwLock::new(vec![1, 2, 3]));
    let readers: Vec<_> = (0..4)
        .map(|_| {
            let rw = Arc::clone(&rw);
            thread::spawn(move || (0..500).map(|_| rw.read().unwrap().len()).sum::<usize>())
        })
        .collect();
    for _ in 0..200 {
        rw.write().unwrap().push(0);
        rw.write().unwrap().pop();
    }
    for r in readers {
        ensure(r.join().unwrap() == 1500, || "reader saw a torn vector".into())?;
    }
    Ok(())
}

fn channels_once_park() -> R {
    let (tx, rx) = mpsc::channel();
    for i in 0..4 {
        let tx = tx.clone();
        thread::spawn(move || {
            for j in 0..100 {
                tx.send(i * 100 + j).unwrap();
            }
        });
    }
    drop(tx);
    let sum: u32 = rx.iter().sum();
    ensure(sum == (0..400).sum(), || format!("channel sum {sum}"))?;

    static ONCE: Once = Once::new();
    static CELL: OnceLock<usize> = OnceLock::new();
    static RUNS: AtomicUsize = AtomicUsize::new(0);
    let hs: Vec<_> = (0..6)
        .map(|i| {
            thread::spawn(move || {
                ONCE.call_once(|| {
                    thread::sleep(Duration::from_millis(20));
                    RUNS.fetch_add(1, Ordering::SeqCst);
                });
                *CELL.get_or_init(|| i)
            })
        })
        .collect();
    let vals: Vec<usize> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    ensure(RUNS.load(Ordering::SeqCst) == 1, || "Once ran more than once".into())?;
    ensure(vals.iter().all(|&v| v == vals[0]), || "OnceLock gave different values".into())?;

    let t = thread::spawn(|| {
        thread::park();
        7
    });
    thread::sleep(Duration::from_millis(30));
    t.thread().unpark();
    ensure(t.join().unwrap() == 7, || "park".into())
}

fn thread_locals() -> R {
    static DROPPED: AtomicUsize = AtomicUsize::new(0);
    struct D;
    impl Drop for D {
        fn drop(&mut self) {
            DROPPED.fetch_add(1, Ordering::SeqCst);
        }
    }
    thread_local! {
        static COUNT: Cell<u32> = const { Cell::new(0) };
        static GUARD: D = const { D };
    }
    let hs: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                GUARD.with(|_| {});
                for _ in 0..=i {
                    COUNT.with(|c| c.set(c.get() + 1));
                }
                COUNT.with(|c| c.get())
            })
        })
        .collect();
    let got: Vec<u32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    ensure(got == [1, 2, 3, 4], || format!("per-thread values {got:?}"))?;
    let d = DROPPED.load(Ordering::SeqCst);
    ensure(d == 4, || format!("{d} of 4 thread-local destructors ran"))
}

fn hashmap_and_random() -> R {
    let mut m = HashMap::new();
    for i in 0..1000 {
        m.insert(format!("key{i}"), i);
    }
    ensure(m.len() == 1000 && m["key500"] == 500, || "HashMap".into())?;
    let a = std::collections::hash_map::RandomState::new();
    let b = std::collections::hash_map::RandomState::new();
    use std::hash::BuildHasher;
    ensure(a.hash_one(1u32) != b.hash_one(1u32), || "RandomState keys repeat".into())
}

fn files() -> R {
    let dir = Path::new("STDTEST.DIR");
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir.join("a").join("b")).map_err(at("fs::create_dir_all"))?;
    let p = dir.join("a").join("data.bin");
    {
        let mut f = fs::File::create(&p).map_err(at("fs::File"))?;
        f.write_all(&[7u8; 100_000]).map_err(at("f.write_all"))?;
        f.seek(SeekFrom::Start(99_990)).map_err(at("f.seek"))?;
        f.write_all(b"end").map_err(at("f.write_all"))?;
        let pos = f.stream_position().map_err(at("f.stream_position"))?;
        ensure(pos == 99_993, || format!("position {pos}"))?;
        f.set_len(200_000).map_err(at("f.set_len"))?;
        f.sync_all().map_err(at("f.sync_all"))?;
    }
    let md = fs::metadata(&p).map_err(at("fs::metadata"))?;
    ensure(md.len() == 200_000 && md.is_file(), || format!("metadata len {}", md.len()))?;
    let _ = md.modified().map_err(at("md.modified"))?;
    let mut buf = Vec::new();
    fs::File::open(&p).map_err(at("fs::File"))?.read_to_end(&mut buf).map_err(at("fs::File"))?;
    ensure(buf.len() == 200_000 && &buf[99_990..99_993] == b"end", || "read back".into())?;

    fs::write(dir.join("b.txt"), "hello").map_err(at("fs::write"))?;
    fs::rename(dir.join("b.txt"), dir.join("c.txt")).map_err(at("fs::rename"))?;
    ensure(fs::read_to_string(dir.join("c.txt")).map_err(at("fs::read_to_string"))? == "hello", || "rename".into())?;
    fs::copy(dir.join("c.txt"), dir.join("a").join("b").join("d.txt")).map_err(at("fs::copy"))?;

    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(e)?
        .map(|d| d.map(|d| d.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .map_err(e)?;
    names.sort();
    ensure(names == ["a", "c.txt"], || format!("read_dir {names:?}"))?;

    let canon = fs::canonicalize(dir.join("a").join("..").join("c.txt")).map_err(|x| format!("canonicalize: {x}"))?;
    let c = canon.to_string_lossy().to_uppercase();
    ensure(c.starts_with(r"\\?\") && c.ends_with(r"\STDTEST.DIR\C.TXT"), || format!("canonicalize gave {}", canon.display()))?;

    let mut perms = fs::metadata(dir.join("c.txt")).map_err(at("fs::metadata"))?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(dir.join("c.txt"), perms.clone()).map_err(|x| format!("set_permissions: {x}"))?;
    ensure(fs::metadata(dir.join("c.txt")).map_err(at("fs::metadata"))?.permissions().readonly(), || "readonly".into())?;
    perms.set_readonly(false);
    fs::set_permissions(dir.join("c.txt"), perms).map_err(at("fs::set_permissions"))?;

    fs::remove_dir_all(dir).map_err(|x| format!("remove_dir_all: {x}"))?;
    ensure(!dir.exists(), || "directory still there".into())
}

fn environment() -> R {
    std::env::set_var("RUST_NT4_TEST", "1");
    ensure(std::env::var("RUST_NT4_TEST").as_deref() == Ok("1"), || "env var".into())?;
    let n = std::env::vars().count();
    ensure(n > 3, || format!("{n} environment variables"))?;
    let cwd = std::env::current_dir().map_err(e)?;
    ensure(cwd.is_absolute(), || format!("current_dir {}", cwd.display()))?;
    let exe = std::env::current_exe().map_err(e)?;
    ensure(exe.to_string_lossy().to_uppercase().ends_with("STDTEST.EXE"), || format!("current_exe {}", exe.display()))?;
    let args: Vec<String> = std::env::args().collect();
    ensure(args.len() == 3 && args[1] == "one" && args[2] == "two words", || format!("args {args:?}"))?;
    let tmp = std::env::temp_dir();
    ensure(tmp.is_dir(), || format!("temp_dir {}", tmp.display()))?;
    let _ = std::env::home_dir();
    Ok(())
}

fn processes() -> R {
    let out = Command::new("cmd").args(["/c", "echo", "from-child"]).output().map_err(e)?;
    let s = String::from_utf8_lossy(&out.stdout);
    ensure(out.status.success() && s.trim() == "from-child", || format!("output {s:?} status {:?}", out.status))?;
    // NT 4.0's cmd has no `exit <code>` (Windows 2000): a failing command gives 1.
    let st = Command::new("cmd").args(["/c", "type", "NO-SUCH.FIL"]).stderr(std::process::Stdio::null()).status().map_err(e)?;
    ensure(st.code() == Some(1), || format!("exit code {:?}", st.code()))?;
    let child = Command::new("cmd").args(["/c", "exit", "0"]).spawn().map_err(e)?;
    let id = child.id();
    let st = child.wait_with_output().map_err(e)?.status;
    ensure(id != 0 && st.success(), || format!("id {id}"))
}

fn time() -> R {
    let t0 = Instant::now();
    thread::sleep(Duration::from_millis(120));
    let dt = t0.elapsed();
    ensure(dt >= Duration::from_millis(110) && dt < Duration::from_secs(2), || format!("slept {dt:?}"))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(e)?;
    ensure(now.as_secs() > 1_600_000_000, || format!("system time {now:?}"))
}

fn network() -> R {
    let l = TcpListener::bind("127.0.0.1:0").map_err(|x| format!("bind: {x}"))?;
    let addr = l.local_addr().map_err(e)?;
    let t = thread::spawn(move || -> std::io::Result<String> {
        let (mut s, _) = l.accept()?;
        let mut b = String::new();
        s.read_to_string(&mut b)?;
        Ok(b)
    });
    let mut c = TcpStream::connect(addr).map_err(|x| format!("connect: {x}"))?;
    c.write_all(b"ping").map_err(e)?;
    drop(c);
    let got = t.join().unwrap().map_err(e)?;
    ensure(got == "ping", || format!("got {got:?}"))?;
    let resolved: Vec<_> = ("localhost", 80).to_socket_addrs().map_err(|x| format!("resolve: {x}"))?.collect();
    ensure(resolved.iter().any(|a| a.ip().is_loopback()), || format!("localhost -> {resolved:?}"))
}

fn float_maths() -> R {
    let x = std::hint::black_box(0.5f64);
    let ok = (x.sin() - 0.479_425_538_604_203).abs() < 1e-12
        && (x.exp() - 1.648_721_270_700_128).abs() < 1e-12
        && (x.powf(2.5) - 0.176_776_695_296_636_9).abs() < 1e-12
        && (x.ln() + 0.693_147_180_559_945_3).abs() < 1e-12
        && (std::hint::black_box(7.5f64) % 2.0 - 1.5).abs() < 1e-12
        && (x.mul_add(4.0, 1.0) - 3.0).abs() < 1e-12
        && x.hypot(1.2) > 1.29
        && std::hint::black_box(2.5f64).round() == 3.0
        && (std::hint::black_box(0.3f32).cos() - 0.955_336_5).abs() < 1e-6;
    ensure(ok, || "a result is off".into())
}

#[link(name = "advapi32")]
extern "system" {
    // RtlGenRandom (nt4compat's on NT 4.0).
    fn SystemFunction036(buffer: *mut u8, length: u32) -> u8;
}

fn random_bytes() -> R {
    let gen = |n: usize| {
        let mut v = vec![0u8; n];
        let ok = unsafe { SystemFunction036(v.as_mut_ptr(), n as u32) };
        (ok, v)
    };
    let t = Instant::now();
    let (ok, a) = gen(65536);
    let first = t.elapsed();
    let (_, b) = gen(65536);
    ensure(ok == 1 && a != b, || "RtlGenRandom failed or repeated".into())?;
    // Monobit: about half the 524288 bits are ones (5 standard deviations = 1810).
    let ones: u32 = a.iter().map(|x| x.count_ones()).sum();
    ensure((ones as i64 - 262_144).abs() < 1810, || format!("{ones} one bits of 524288"))?;
    // Byte frequencies: chi-square with 255 degrees of freedom, far below 400.
    let mut f = [0u32; 256];
    for &x in &a {
        f[x as usize] += 1;
    }
    let chi: f64 = f.iter().map(|&o| (o as f64 - 256.0).powi(2) / 256.0).sum();
    ensure(chi < 400.0, || format!("chi-square {chi:.0}"))?;
    println!("    (first call, gathering included: {first:?})");
    Ok(())
}

fn main() {
    let v = std::process::Command::new("cmd").args(["/c", "ver"]).output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    println!("std on Windows NT 4.0 ({v})");
    let mut failures = 0;
    check(&mut failures, "threads, Mutex", threads_and_mutex);
    check(&mut failures, "Condvar, RwLock", condvar_and_rwlock);
    check(&mut failures, "channels, Once, park", channels_once_park);
    check(&mut failures, "thread_local + destructors", thread_locals);
    check(&mut failures, "HashMap, random keys", hashmap_and_random);
    check(&mut failures, "RtlGenRandom quality", random_bytes);
    check(&mut failures, "files and directories", files);
    check(&mut failures, "environment, args", environment);
    check(&mut failures, "processes", processes);
    check(&mut failures, "time, sleep", time);
    check(&mut failures, "network (TCP, resolve)", network);
    check(&mut failures, "float maths", float_maths);
    if failures == 0 {
        println!("stdtest: ok");
    } else {
        println!("stdtest: {failures} FAILED");
    }
    std::process::exit(failures as i32);
}
