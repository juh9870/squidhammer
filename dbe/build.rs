//! This is required to trigger a rebuild when localization files change

fn main() {
    println!("cargo:rerun-if-changed=locales");
}
