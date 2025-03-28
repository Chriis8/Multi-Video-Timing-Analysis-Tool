mod video_pipeline;
use gtk::{gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, CssProvider, Window};
use gstgtk4;
mod widgets;
use widgets::video_player_widget::video_player::VideoPlayer;

fn load_css() {
    let provider = CssProvider::new();
    match std::env::current_dir() {
        Ok(current_dir) => {
            let file = gio::File::for_path(current_dir.join("src\\widgets\\main_window\\style.css"));
            provider.load_from_file(&file);
        }
        Err(e) => {
            eprintln!("Failed to get current working directory to load css ({e})");
        }
    }
    if let Some(display) = Display::default() {
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
}

fn build_ui(app: &Application) {
    let builder = Builder::from_resource("/mainwindow/mwindow.ui");
    load_css();

    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");

    let video_container: Box = builder.object("video_container").expect("failed to get video_container from UI file");

    let player1 = VideoPlayer::new();
    let player2 = VideoPlayer::new();
    video_container.append(&player1);
    video_container.append(&player2);
    
    let window_clone = window.clone();
    player1.setup_event_handlers(window_clone);

    let window_clone = window.clone();
    player2.setup_event_handlers(window_clone);

    app.add_window(&window);
    window.show();
}

fn main() -> glib::ExitCode {
    gstreamer::init().unwrap();
    gtk::init().unwrap();

    std::env::set_var("GTK_THEME", "Adwaita:dark");

    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    gio::resources_register_include!("vplayer.gresource")
        .expect("Failed to register resources.");

    gio::resources_register_include!("mwindow.gresource")
        .expect("Failed to register resources.");
    
    let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);
    app.connect_activate(|app| {

        // let window = gtk::ApplicationWindow::new(app);
        
        // window.set_default_size(640, 480);
        // window.set_title(Some("Video Player"));

        // let player1 = VideoPlayer::new();
        // let player2 = VideoPlayer::new();

        // let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        // container.append(&player1);
        // container.append(&player2);
        
        // window.set_child(Some(&container));

        // player1.setup_event_handlers();
        // player2.setup_event_handlers();

        // app.add_window(&window);

        // window.show();
        build_ui(app);
    });

    let res = app.run();

    unsafe {
        gstreamer::deinit();
    }
    res
}
