mod video_pipeline;
use glib::{random_int_range, ExitCode, prelude::ObjectExt};
use gtk::{FlowBox, FlowBoxChild, SelectionMode, SingleSelection,};
use gtk::{glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button};
use gstgtk4;
mod widgets;
use widgets::seek_bar::seek_bar::SeekBar;
use widgets::seek_bar::shared_seek_bar::{self, SharedSeekBar};
use widgets::split_panel::splittable::SplitTable;
use widgets::video_player_widget::video_player::VideoPlayer;
use widgets::split_panel::splits::VideoSegment;
use gtk::prelude::GtkWindowExt;
mod helpers;
use crate::helpers::data::{get_data, store_data};
use crate::helpers::ui::load_css;
use crate::helpers::ui::flowbox_children;
use crate::widgets::split_panel::timeentry::TimeEntry;
use std::cell::Cell;

const MAX_VIDEO_PLAYERS: u32 = 6;

fn build_ui(app: &Application) -> Builder {
    let builder = Builder::from_resource("/mainwindow/mwindow.ui");    

    load_css("src\\widgets\\main_window\\style.css");
    load_css("src\\widgets\\split_panel\\style.css");
    
    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
    let column_view_container: Box = builder.object("split_container").expect("Failed to column_view_container from UI File");
    let video_container: FlowBox = builder.object("video_container").expect("Failed to get video_container from UI File");
    let add_row_above_button: Button = builder.object("add_row_above_button").expect("Failed to get add_row_above_button from UI File");
    let add_row_below_button: Button = builder.object("add_row_below_button").expect("Failed to get add_row_below_button from UI File");
    let bottom_vbox: Box = builder.object("bottom_vbox").expect("Failed to get bottom_vbox from UI File");
    let start_time_offset_container: Box = builder.object("start_time_offset_container").expect("Failed to get start_time_offset_container from UI File");
    
    //Testing Stuff
    let test_button: Button = builder.object("test_button").expect("failed to get test button");
    let test_button2: Button = builder.object("test_button2").expect("failed to get test button2");
    let test_button3: Button = builder.object("test_button3").expect("failed to get test button3");
    
    video_container.set_homogeneous(true);
    video_container.set_valign(gtk::Align::Fill);
    video_container.set_selection_mode(SelectionMode::None);
    video_container.set_column_spacing(0);
    
    let split_table = SplitTable::new();
    let split_table_cv = split_table.get_split_table_column_view().unwrap();
    let split_table_ls = split_table.get_split_table_liststore().unwrap();
    let start_time_offset_ls = split_table.get_start_time_offset_liststore().unwrap();
    split_table.set_max_number_of_video_players(MAX_VIDEO_PLAYERS);
    column_view_container.append(&split_table_cv);

    let start_time_offset_cv = split_table.get_start_time_offset_column_view().unwrap();
    start_time_offset_container.append(&start_time_offset_cv);

    split_table.setup_start_time_offset_column("Start Time Offsets");

    let ssb = SharedSeekBar::new(&video_container, &split_table_cv, &start_time_offset_ls, &split_table_ls);
    bottom_vbox.append(&ssb);
    
    // Adds an initial row
    //split_table.append_empty_row();
    //split_table.connect_row_to_seekbar(&video_container, 0);
    
    // Adds first row of segment names to the split table
    split_table.add_name_column("Segment Name");

    // Add data to video_container to keep track of the number of active videos
    let initial_child_count = 0_usize;
    store_data(&video_container, "count", initial_child_count);

    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = ssb.clone();
    let split_table_clone = split_table.clone();
    add_row_above_button.connect_clicked(move |_| {
        let selection_model = split_table_clone.get_split_table_column_view()
            .unwrap()
            .model()
            .and_downcast::<SingleSelection>()
            .unwrap();
        let mut selected_index = 0u32;
        if let Some(_selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            selected_index = selection_model.selected();
        }
        split_table_clone.insert_empty_row(selected_index);
        split_table_clone.connect_row_to_seekbar(&video_container_clone, selected_index);
        let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
        shared_seek_bar_clone.connect_row(selected_index, video_player_count as u32);
    });

    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = ssb.clone();
    let split_table_clone = split_table.clone();
    add_row_below_button.connect_clicked(move |_| {
        let selection_model = split_table_clone.get_split_table_column_view()
            .unwrap()
            .model()
            .and_downcast::<SingleSelection>()
            .unwrap();
        let mut selected_index = 0u32;
        if let Some(_selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            selected_index = selection_model.selected() + 1;
        }
        split_table_clone.insert_empty_row(selected_index);
        split_table_clone.connect_row_to_seekbar(&video_container_clone, selected_index);
        let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
        shared_seek_bar_clone.connect_row(selected_index, video_player_count as u32);
    });
    
    let new_video_player_button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = ssb.clone();
    
    // Adds new video player and new columns to split table
    let split_table_clone = split_table.clone();
    new_video_player_button.connect_clicked(move |_| {
        let count = *unsafe{ get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() };
        if count as u32 == MAX_VIDEO_PLAYERS {
            println!("Max video players reached");
            return;
        }

        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        
        // Sets up new video player
        let new_player = VideoPlayer::new(count as u32);
        new_player.setup_event_handlers(window);
        
        let split_table_clone_clone = split_table_clone.clone();
        // Listens to the split button from a video player
        // args[1] ID u32: index from the video player thats button was pressed
        // args[2] Position u64: time in nano seconds that the video player playback head was at when the button was pressed
        new_player.connect_local("split-button-clicked", false, move |args| {
            let video_player_index: u32 = args[1].get().unwrap();
            let video_player_position: u64 = args[2].get().unwrap();
            // Sets the time for the selected row
            if let Err(e) = split_table_clone_clone.set_split(video_player_index, video_player_position) {
                eprintln!("{e}");
            }
            None
        });

        let split_table_clone_clone = split_table_clone.clone();
        new_player.connect_local("set-start-button-clicked", false, move |args| {
            let video_player_index: u32 = args[1].get().unwrap();
            let video_player_position: u64 = args[2].get().unwrap();
            if let Err(e) = split_table_clone_clone.set_start_time_offset(video_player_index, video_player_position) {
                eprintln!("{e}");
            }
            None
        });

        // Adds start time offset entry text to start_time_offset liststore/columnview
        let new_start_time_offset_time_entry = match split_table_clone.add_start_time_offset_row() {
            Ok(te) => te,
            Err(e) => {
                panic!("{e}")
            }
        };

        new_start_time_offset_time_entry.connect_notify_local(Some("time"), glib::clone!(
            #[weak(rename_to = shared_seek_bar)] shared_seek_bar_clone,
            move |_, _| {
                shared_seek_bar.update_timeline_length();
        }));
        
        // Adds two columns to split table for each new video player
        // Column 1: (Time) Split time -> time since the start of the clip
        // Column 2: (Duration) Segment time -> time since the last split
        let name = random_int_range(0, 99);
        split_table_clone.add_column(name.to_string().as_str(), count as u32, &format!("relative-time-{}", count));
        split_table_clone.add_column(name.to_string().as_str(), count as u32, &format!("duration-{}", count));

        // Updates formatting of the video players and adds the new video player to the container
        let number_of_columns = (count as u32 + 1).clamp(1,3);
        video_container_clone.set_max_children_per_line(number_of_columns);
        video_container_clone.set_min_children_per_line(number_of_columns);
        video_container_clone.append(&new_player);
        
        let video_player_index = count as u32;
        // Updates video_container data keeping track of the active video players
        store_data(&video_container_clone, "count", count + 1);

        //split_table.connect_column_to_seekbar(&video_container_clone, video_player_index);
        let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
        shared_seek_bar_clone.connect_column(video_player_index, video_player_count as u32);
    });

    let video_container_clone = video_container.clone();
    test_button.connect_clicked(move |_| {
        for (video_player_index, child) in flowbox_children(&video_container_clone).enumerate() {
            let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                Some(c) => c,
                None => continue,
            };

            let content = match fb_child.child() {
                Some(c) => c,
                None => continue,
            };

            let video_player = match content.downcast_ref::<VideoPlayer>() {
                Some(vp) => vp,
                None => continue,
            };

            let arc = match video_player.pipeline().upgrade() {
                Some(a) => a,
                None => {
                    eprintln!("Shared jump to segment: Pipeline dropped");
                    continue
                }
            };

            let mut guard = match arc.lock() {
                Ok(g) => g,
                Err(_) => {
                    eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                    continue
                }
            };

            if let Some(pipeline) = guard.as_mut() {
                let time = pipeline.get_position().unwrap();
                println!("Player {video_player_index} at position: {time}");
            } else {
                eprintln!("No pipeline for index {video_player_index}");
            }
        }
    });

    let split_table_clone = split_table.clone();
    let video_container_clone = video_container.clone();
    test_button2.connect_clicked(move |_| {
        let offset_liststore = split_table_clone.get_start_time_offset_liststore().unwrap();
        for (video_player_index, child) in flowbox_children(&video_container_clone).enumerate() {
            let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                Some(c) => c,
                None => continue,
            };

            let content = match fb_child.child() {
                Some(c) => c,
                None => continue,
            };

            let video_player = match content.downcast_ref::<VideoPlayer>() {
                Some(vp) => vp,
                None => continue,
            };

            let arc = match video_player.pipeline().upgrade() {
                Some(a) => a,
                None => {
                    eprintln!("Shared jump to segment: Pipeline dropped");
                    continue
                }
            };

            let mut guard = match arc.lock() {
                Ok(g) => g,
                Err(_) => {
                    eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                    continue
                }
            };
            
            if let Some(pipeline) = guard.as_mut() {
                let offset = offset_liststore.item(video_player_index as u32).and_downcast::<TimeEntry>().unwrap();
                let start_time = gstreamer::ClockTime::from_nseconds(offset.get_time());
                if let Err(e) = pipeline.seek_position(start_time) {
                    eprintln!("Player {video_player_index} error setting position: {e}");
                }
            } else {
                eprintln!("No pipeline for index {video_player_index}");
            }
        }
    });

    let shared_seek_bar_clone = ssb.clone();
    test_button3.connect_clicked(move |_| {
        shared_seek_bar_clone.toggle_has_control();
    });

    app.add_window(&window);
    window.show();
    builder
}

fn main() -> glib::ExitCode {
    let run_app = 0;
    if run_app == 0 {
        gstreamer::init().unwrap();
        gtk::init().unwrap();

        std::env::set_var("GTK_THEME", "Adwaita:dark");

        gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

        gio::resources_register_include!("vplayer.gresource")
            .expect("Failed to register video player resource.");

        gio::resources_register_include!("mwindow.gresource")
            .expect("Failed to register main window resource.");

        // gio::resources_register_include!("spanel.gresource")
        //     .expect("Failed to register split planel resource.");

        gio::resources_register_include!("seekbar.gresource")
            .expect("Failed to register seek bar resource.");

        gio::resources_register_include!("sharedseekbar.gresource")
            .expect("Failed to register shared seek bar resource.");

        gio::resources_register_include!("sptable.gresource")
            .expect("Failed to register sptable resource");
        
        let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);
        app.connect_activate(|app| {
            
            let builder = build_ui(app);

            let builder_clone = builder.clone();
            // ensures all video player are properly disposed 
            app.connect_shutdown(move |_| {
                println!("shutting down");
                let video_container: FlowBox = builder_clone.object("video_container").expect("failed to get video_container from UI file");
                while let Some(child) = video_container.last_child() {
                    let video = child.downcast::<FlowBoxChild>().unwrap();
                    unsafe {
                        video.unparent(); 
                        video.run_dispose();
                    }
                }
            });

        });
        let res = app.run();

        unsafe {
            gstreamer::deinit();
        }
        return res
    } else if run_app == 1 {
        gstreamer::init().unwrap();
        gtk::init().unwrap();

        std::env::set_var("GTK_THEME", "Adwaita:dark");

        gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

        gio::resources_register_include!("vplayer.gresource")
            .expect("Failed to register video player resource.");

        gio::resources_register_include!("mwindow.gresource")
            .expect("Failed to register main window resource.");

        // gio::resources_register_include!("spanel.gresource")
        //     .expect("Failed to register split planel resource.");

        gio::resources_register_include!("seekbar.gresource")
            .expect("Failed to register seek bar resource.");

        gio::resources_register_include!("sharedseekbar.gresource")
            .expect("Failed to register shared seek bar resource.");

        gio::resources_register_include!("sptable.gresource")
            .expect("Failed to register sptable resource");
        
        let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);


        app.connect_activate(|app| {

            let window = ApplicationWindow::new(app);

            window.set_default_size(800, 600);
            window.set_title(Some("Video Player"));

            load_css("src\\widgets\\main_window\\style.css");

            let main_box = Box::new(gtk::Orientation::Horizontal, 10);
            
            window.set_child(Some(&main_box));
            window.show();
        });


        let res = app.run();

        unsafe {
            gstreamer::deinit();
        }
        return res
    }
    ExitCode::SUCCESS
}