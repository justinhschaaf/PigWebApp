use std::path::Path;
use std::process::Command;

const CLIENT_DIR: &str = "../client";

// Compile the client before the server whenever it changes
// https://theadventuresofaliceandbob.com/posts/rust_rocket_yew_part1.md
fn main() {
    println!("cargo:rerun-if-changed={}/src", CLIENT_DIR);
    println!("cargo:rerun-if-changed={}/index.html", CLIENT_DIR);
    build_client(CLIENT_DIR);
}

fn build_client<P: AsRef<Path>>(source: P) {
    Command::new("trunk")
        .args(&["build", "--release"])
        .current_dir(source.as_ref())
        .status()
        .expect("Failed to build client");
}
