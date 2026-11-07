//! GnuTLS on NT 4.0 (see Cargo.toml): its random numbers (RtlGenRandom: nt4compat's
//! software entropy), a certificate made here, and a TLS handshake between a server
//! thread and a client thread over a loopback connection, with a message each way.
use std::ffi::{c_char, c_int, CStr};
use std::net::{TcpListener, TcpStream};
use std::os::windows::io::AsRawSocket;
use std::thread;

extern "C" {
    fn tls_version(out: *mut c_char, size: c_int) -> c_int;
    fn tls_random(out: *mut u8, n: c_int) -> c_int;
    fn tls_setup_server() -> c_int;
    fn tls_server(s: usize, info: *mut c_char, size: c_int) -> c_int;
    fn tls_client(s: usize, info: *mut c_char, size: c_int) -> c_int;
    fn tls_error(r: c_int) -> *const c_char;
    fn tls_debug(level: c_int) -> c_int;
}

fn err(r: c_int) -> String {
    unsafe { CStr::from_ptr(tls_error(r)) }.to_string_lossy().into_owned()
}

fn end(f: unsafe extern "C" fn(usize, *mut c_char, c_int) -> c_int, s: &TcpStream) -> (c_int, String) {
    let mut info = [0 as c_char; 128];
    let r = unsafe { f(s.as_raw_socket() as usize, info.as_mut_ptr(), 128) };
    (r, unsafe { CStr::from_ptr(info.as_ptr()) }.to_string_lossy().into_owned())
}

fn main() {
    let mut failures = 0;
    let mut v = [0 as c_char; 32];
    unsafe { tls_version(v.as_mut_ptr(), 32) };
    println!("GnuTLS {} on Windows NT 4.0", unsafe { CStr::from_ptr(v.as_ptr()) }.to_string_lossy());

    let level = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(0);
    if level > 0 {
        let v = unsafe { tls_debug(level) };
        eprintln!("thread-local read back: {v}");
    }
    let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
    let r = unsafe { tls_random(a.as_mut_ptr(), 32) + tls_random(b.as_mut_ptr(), 32) };
    let ok = r == 0 && a != b && a != [0; 32];
    println!("  {:<24} {}", "gnutls_rnd", if ok { "ok" } else { "FAILED" });
    failures += !ok as i32;

    let t = std::time::Instant::now();
    let r = unsafe { tls_setup_server() };
    println!("  {:<24} {}", "ECDSA key, certificate", if r == 0 { format!("ok ({:?})", t.elapsed()) } else { format!("FAILED: {}", err(r)) });
    if r != 0 {
        std::process::exit(1);
    }

    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        end(tls_server, &s)
    });
    let c = TcpStream::connect(addr).unwrap();
    let t = std::time::Instant::now();
    let (rc, ic) = end(tls_client, &c);
    let (rs, is) = server.join().unwrap();
    let ok = rc == 0 && rs == 0;
    println!(
        "  {:<24} {}",
        "TLS handshake + data",
        if ok { format!("ok: {ic} ({:?})", t.elapsed()) } else { format!("FAILED: client {rc} {ic}; server {rs} {is}") }
    );
    failures += !ok as i32;
    println!("tlstest: {}", if failures == 0 { "ok".to_string() } else { format!("{failures} FAILED") });
    std::process::exit(failures);
}
