use std::env;

#[cfg(target_os = "macos")]
fn find_dependencies() {
    let s: String;
    match env::var("HOMEBREW_CELLAR") {
        Ok(val) => s = String::from(val),
        Err(e) => panic!("{e}")
    };
    println!("cargo::rustc-link-arg-bin=chip8_frontend=-lSDL2 -L{}", s);
}

#[cfg(target_os = "windows")]
fn find_dependencies() {
}

#[cfg(target_os = "linux")]
fn find_dependencies() {
}

fn main() {
    find_dependencies();
}