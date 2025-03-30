mod video_pipeline;
use gtk::{gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, CssProvider, Button};
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

fn build_ui(app: &Application) -> Builder {
    let builder = Builder::from_resource("/mainwindow/mwindow.ui");
    load_css();

    // let button: Button = builder.object("new_video_player_button").expect("Failed to get button from UI file");

    // let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
    // let video_container: Box = builder.object("video_container").expect("Failed to get video_container from UI File");
    // button.connect_clicked(glib::clone!(
    //     #[weak(rename_to = window_clone)] window,
    //     #[weak(rename_to = video_container_clone)] video_container,
    //     move |_| {
    //         let new_player = VideoPlayer::new();
    //         new_player.setup_event_handlers(window_clone);
    //         video_container_clone.append(&new_player);
    //     }
    // ));
    
    
    
    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");

    //let video_container: Box = builder.object("video_container").expect("failed to get video_container from UI file");

    // let player1 = VideoPlayer::new();
    // let player2 = VideoPlayer::new();
    // let player3 = VideoPlayer::new();
    // video_container.append(&player1);
    // video_container.append(&player2);
    // video_container.append(&player3);
    
    // let window_clone = window.clone();
    // player1.setup_event_handlers(window_clone);

    // let window_clone = window.clone();
    // player2.setup_event_handlers(window_clone);

    // let window_clone = window.clone();
    // player3.setup_event_handlers(window_clone);

    app.add_window(&window);
    window.show();
    builder
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
        let builder = build_ui(app);
        set_up_button(builder);
    });

    let res = app.run();

    unsafe {
        gstreamer::deinit();
    }
    res
}

fn set_up_button(builder: Builder) {
    let button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    
    button.connect_clicked(move |_| {
        let video_container: Box = builder.object("video_container").expect("failed to get video_container from UI file");
        let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
        let new_player = VideoPlayer::new();
        new_player.setup_event_handlers(window);
        video_container.append(&new_player);
    });
}