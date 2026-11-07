use std::env;

fn main() {
    let deps = env::var("NT4_DEPS").expect("NT4_DEPS: the prefix of GnuTLS and its libraries");
    println!("cargo:rerun-if-changed=c/tls.c");
    println!("cargo:rerun-if-env-changed=NT4_DEPS");
    cc::Build::new().file("c/tls.c").include(format!("{deps}/include")).compile("tls");
    println!("cargo:rustc-link-search=native={deps}/lib");
    for lib in ["libgnutls", "libhogweed", "libnettle", "libgmp", "libtasn1", "libiconv"] {
        println!("cargo:rustc-link-arg={deps}/lib/{lib}{}", if lib.contains("nettle") || lib.contains("hogweed") { ".a" } else { ".lib" });
    }
    println!("cargo:rustc-link-lib=ws2_32");
}
