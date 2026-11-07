fn main() {
    println!("cargo:rerun-if-changed=c/mix.c");
    cc::Build::new().file("c/mix.c").compile("mix");
}
