use std::path::PathBuf;
use std::env;

fn main() {
    if cfg!(target_os = "windows") {
        let out_dir = env::var("OUT_DIR").unwrap();
        let target_icon = PathBuf::from(&out_dir).join("icon.ico");
        
        let img = image::open("assets/icon.png").expect("Failed to open icon.png");
        img.save_with_format(&target_icon, image::ImageFormat::Ico).expect("Failed to save icon.ico");
        
        let mut res = winres::WindowsResource::new();
        res.set_icon(target_icon.to_str().unwrap());
        res.compile().unwrap();
    }
}
