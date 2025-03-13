fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    println!("cargo:warning=OUT_DIR is: {}", out_dir);
    
    glib_build_tools::compile_resources(
        &["src/widgets"], 
        "src/widgets/video_player.gresource.xml", 
        "video_player.gresource");
    
}
