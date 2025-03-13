mod video_pipeline;
use gstreamer::prelude::*;
use gtk::{prelude::*, Button, glib};
use std::cell::RefCell;
use gstgtk4;
use std::sync::Arc;
mod widgets;
use widgets::video_player_widget::VideoPlayer;

fn create_ui(app: &gtk::Application, gstreamer_manager: Arc<RefCell<video_pipeline::VideoPipeline>>) {
    let window = gtk::ApplicationWindow::new(app);
    

    window.set_default_size(640, 480);
    window.set_title(Some("Video Player"));

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let hboxtop = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let picture = gtk::Picture::new();
    let label = gtk::Label::new(Some("Position: 00:00:00"));

    let text_view = gtk::Label::new(Some("Please, select a mpegts video file."));
    let fchooser = Button::with_label("Open File");

    fchooser.connect_clicked({
        let gstreamer_manager_clone = gstreamer_manager.clone();
        let text_view = text_view.clone();
        let window = window.clone();
        let picture_clone = picture.clone();
    
        move |_| {
            println!("Done");
    
            let videos_filter = gtk::FileFilter::new();
            videos_filter.set_name(Some("MPEGTS"));
    
            let dialog = gtk::FileChooserDialog::builder()
                .title("Open File")
                .action(gtk::FileChooserAction::Open)
                .modal(true)
                .build();
    
            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Accept", gtk::ResponseType::Accept);
            dialog.set_transient_for(Some(&window));
    
            let gstreamer_manager_double_clone = gstreamer_manager_clone.clone();
            let text_view = text_view.clone();
            let picture_double_clone = picture_clone.clone();
    
            dialog.run_async(move |obj, res| {
                match res {
                    gtk::ResponseType::Accept => {
                        println!("Accepted");
                        if let Some(file) = obj.file() {
                            let from_str = gtk::gio::File::uri(&file);
                            println!("from_str {from_str}");
                            text_view.set_label(&from_str);
                            println!("File accepted: {}", from_str);
                            gstreamer_manager_double_clone.borrow_mut().reset();

                            gstreamer_manager_double_clone.borrow().build_pipeline(Some(&text_view.label().to_string()));

                            let paintable = gstreamer_manager_double_clone.borrow().get_paintable();
                            picture_double_clone.set_paintable(Some(&paintable));
                        }
                    }
                    _ => {
                        println!("No file selected");
                    }
                }
                obj.destroy();
            });
        }
    });

    hboxtop.append(&fchooser);
    hboxtop.append(&text_view);  
    hboxtop.set_halign(gtk::Align::Center);
    hboxtop.set_margin_top(20);
  
    vbox.append(&hboxtop);
    vbox.append(&picture);
    vbox.append(&label);
    vbox.append(&hbox);
    
    let previous_frame_button = Button::with_label("Previous Frame");
    previous_frame_button.connect_clicked({
        let gstreamer_manager_clone = gstreamer_manager.clone();

        move |_| {
            gstreamer_manager_clone.borrow().frame_backward();
            eprintln!("Moved 1 frame backward");
        }
    });
    let play_button = Button::with_label("Play");
    play_button.connect_clicked({ 
        let gstreamer_manager_clone = gstreamer_manager.clone();
        move |_| {
            eprintln!("Pressed Play button");
            gstreamer_manager_clone.borrow().play_video();
        }
    });
    let stop_button = Button::with_label("Stop");
    stop_button.connect_clicked({
        let gstreamer_manager_clone = gstreamer_manager.clone();

        move |_| {
            eprintln!("Pressed Stop button");
            gstreamer_manager_clone.borrow().stop_video();
        }
    });
    let next_frame_button = Button::with_label("Next Frame");
    next_frame_button.connect_clicked({
        let gstreamer_manager_clone = gstreamer_manager.clone();

        move |_| {
            gstreamer_manager_clone.borrow().frame_forward();
            eprintln!("Moved 1 frame foward");
        }
    });
    let test_button = Button::with_label("Test");
    test_button.connect_clicked({
        let gstreamer_manager_clone = gstreamer_manager.clone();

        move |_| {
            eprintln!("Testing stuff");
            gstreamer_manager_clone.borrow().get_current_frame();
        }
    });
    
    hbox.append(&previous_frame_button);
    hbox.append(&play_button);
    hbox.append(&stop_button);
    hbox.append(&next_frame_button);
    hbox.append(&test_button);
    hbox.set_halign(gtk::Align::Center);
    hbox.set_margin_bottom(20);
    
    window.set_child(Some(&vbox));
    window.show();
    
    app.add_window(&window);
    
    let gstreamer_manager_weak = Arc::downgrade(&gstreamer_manager);
    let timeout_id = glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let gstreamer_manager_c = match gstreamer_manager_weak.upgrade() {
            Some(gstreamer_manager_c) => gstreamer_manager_c,
            None => return gtk::glib::ControlFlow::Break,
        };

        let position = gstreamer_manager_c.borrow().get_position();
        label.set_text(&format!("Position: {:.0}", position.display()));
        gtk::glib::ControlFlow::Continue
    });    
    
    let bus = gstreamer_manager.borrow().get_bus();
    let app_weak = app.downgrade();
    let gstreamer_manager_clone = gstreamer_manager.clone();
    let bus_watch = bus
        .add_watch_local(move |_, msg| {
            use gstreamer::MessageView;

            let app = match app_weak.upgrade() {
                Some(app) => app,
                None => return gtk::glib::ControlFlow::Break,
            };

            match msg.view() {
                MessageView::Eos(..) => {
                    gstreamer_manager_clone.borrow().pause_video();
                },
                MessageView::Error(err) => {
                    println!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    app.quit();
                }
                _ => (),
            };

            gtk::glib::ControlFlow::Continue
        })
        .expect("Failed to add bus watch");

    let timeout_id = RefCell::new(Some(timeout_id));    
    let bus_watch = RefCell::new(Some(bus_watch));
    app.connect_shutdown(move |_| {
        window.close();

        drop(bus_watch.borrow_mut().take());
        // if let Some(_pipeline) = _pipeline.borrow_mut().take() {
            // gstman.setStopStream();
        // }

        if let Some(timeout_id) = timeout_id.borrow_mut().take() {
            timeout_id.remove();
        }
    });
}

fn main() -> glib::ExitCode {
    gstreamer::init().unwrap();
    gtk::init().unwrap();

    std::env::set_var("GTK_THEME", "Adwaita:dark");

    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

    gio::resources_register_include!("video_player.gresource")
        .expect("Failed to register resources.");

    let gstreamer_manager_1 = Arc::new(RefCell::new(video_pipeline::VideoPipeline::new()));
    //let gstreamer_manager_2 = Arc::new(RefCell::new(video_pipeline::VideoPipeline::new()));
    
    let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);

    app.connect_activate(|app| {
        // let gstreamer_manager_1 = Arc::clone(&gstreamer_manager_1);
        // move |app| {
        //     create_ui(app, gstreamer_manager_1.clone())
        // }

        let window = gtk::ApplicationWindow::new(app);
        
        window.set_default_size(640, 480);
        window.set_title(Some("Video Player"));

        let player1 = VideoPlayer::new();
        let player2 = VideoPlayer::new();


        let container = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        container.append(&player1);
        container.append(&player2);
        
        window.set_child(Some(&container));
        app.add_window(&window);
        
        window.show();

    });

    // app.connect_activate({
    //     let gstreamer_manager_2 = Arc::clone(&gstreamer_manager_2);
    //     move |app| {
    //         create_ui(app, gstreamer_manager_2.clone())
    //     }
    // });


    let res = app.run();

    unsafe {
        gstreamer::deinit();
    }
    res
}
