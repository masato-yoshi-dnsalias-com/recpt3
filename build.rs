use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use serde::Deserialize;
use toml;

#[derive(Deserialize)]
struct Versions {
    versions: HashMap<String, String>,
}

fn main() {
    let mut file = File::open("version.toml").expect("Failed to open version.toml");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read version.toml");

    let versions: Versions = toml::from_str(&contents).expect("Failed to parse ver.toml");

    for (key, value) in versions.versions.iter() {
        println!("cargo:rustc-env=VERSION_{}={}", key.to_uppercase(), value);
    };

    println!("cargo:rustc-link-lib=dylib=arib25");
}
