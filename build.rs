fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    println!("cargo:warning=OUT_DIR is: {}", out_dir);
    
    glib_build_tools::compile_resources(
        &["src/widgets/video_player_widget"], 
        "src/widgets/video_player_widget/vplayer.gresource.xml", 
        "vplayer.gresource");

    glib_build_tools::compile_resources(
        &["src/widgets/main_window"], 
        "src/widgets/main_window/mwindow.gresource.xml",
        "mwindow.gresource");

    glib_build_tools::compile_resources(
        &["src/widgets/split_panel"],
        "src/widgets/split_panel/spanel.gresource.xml", 
        "spanel.gresource");

    glib_build_tools::compile_resources(
        &["src/widgets/video_player_widget"], 
        "src/widgets/video_player_widget/seekbar.gresource.xml", 
        "seekbar.gresource");
    
}
