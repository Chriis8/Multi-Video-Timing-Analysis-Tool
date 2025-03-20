mod video_pipeline;
use gtk::{prelude::*, glib};
use gstgtk4;
mod widgets;
use widgets::video_player_widget::VideoPlayer;

fn main() -> glib::ExitCode {
    gstreamer::init().unwrap();
    gtk::init().unwrap();

    std::env::set_var("GTK_THEME", "Adwaita:dark");

    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    gio::resources_register_include!("video_player.gresource")
        .expect("Failed to register resources.");
    
    let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);
    app.connect_activate(|app| {

        let window = gtk::ApplicationWindow::new(app);
        
        window.set_default_size(640, 480);
        window.set_title(Some("Video Player"));

        let player1 = VideoPlayer::new();
        let player2 = VideoPlayer::new();

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.append(&player1);
        container.append(&player2);
        
        window.set_child(Some(&container));

        player1.setup_event_handlers();
        player2.setup_event_handlers();

        app.add_window(&window);
        
        window.show();
    });

    let res = app.run();

    unsafe {
        gstreamer::deinit();
    }
    res
}
