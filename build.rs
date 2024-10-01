#[cfg(target_os = "macos")]
fn find_dependencies() {
    println!("cargo::rustc-link-arg-bin=chip8_frontend=-lSDL2 -L$HOMEBREW_CELLAR")
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