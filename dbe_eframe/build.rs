use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc_path = out_dir.join("icon.rc");
    let ico_path = out_dir.join("favicon.ico");

    fs_err::write(&rc_path, include_bytes!("../assets/icon.rc")).unwrap();
    fs_err::write(&ico_path, include_bytes!("../assets/favicon.ico")).unwrap();

    embed_resource::compile(rc_path, embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
