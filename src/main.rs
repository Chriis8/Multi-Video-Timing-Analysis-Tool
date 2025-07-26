mod video_pipeline;
use glib::{ExitCode, prelude::ObjectExt};
use gtk::{FlowBox, FlowBoxChild, SelectionMode, SingleSelection,};
use gtk::{glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button};
use gstgtk4;
mod widgets;
use helpers::data::get_next_id;
use widgets::seek_bar::seek_bar::SeekBar;
use widgets::seek_bar::shared_seek_bar::SharedSeekBar;
use widgets::split_panel::splittable::SplitTable;
use widgets::video_player_widget::video_player::VideoPlayer;
use widgets::split_panel::splits::VideoSegment;
use gtk::prelude::GtkWindowExt;
mod helpers;
use crate::helpers::data::{get_data, store_data};
use crate::helpers::ui::load_css;
use crate::helpers::ui::flowbox_children;
use crate::widgets::seek_bar::color_picker::ColorPool;
use crate::widgets::split_panel::timeentry::TimeEntry;
use crate::widgets::sync::sync_manager::SyncManager;
use std::cell::RefCell;
use std::rc::Rc;

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
    let toggle_shared_video_play: Button = builder.object("toggle_shared_video_play").expect("failed to get test button3");
    
    video_container.set_homogeneous(true);
    video_container.set_valign(gtk::Align::Fill);
    video_container.set_selection_mode(SelectionMode::None);
    video_container.set_column_spacing(0);
    
    let split_table = SplitTable::new();
    let split_table_cv = split_table.get_split_table_column_view().unwrap();
    let split_table_ls = split_table.get_split_table_liststore().unwrap();
    let start_time_offset_ls = split_table.get_start_time_offset_liststore().unwrap();
    column_view_container.append(&split_table_cv);

    let start_time_offset_cv = split_table.get_start_time_offset_column_view().unwrap();
    start_time_offset_container.append(&start_time_offset_cv);

    split_table.setup_start_time_offset_column("Start Time Offsets");

    let sync_manager = SyncManager::new();
    store_data(&window, "sync_manager", sync_manager.clone());

    let ssb = SharedSeekBar::new(&video_container, &split_table_cv, &start_time_offset_ls, &split_table_ls, &sync_manager, &split_table);
    bottom_vbox.append(&ssb);

    let shared_seek_bar_clone = ssb.clone();
    sync_manager.add_sync_callback(move |event| {
        shared_seek_bar_clone.handle_sync_event(event);
    });
    
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
        shared_seek_bar_clone.connect_row(selected_index);
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
        shared_seek_bar_clone.connect_row(selected_index);
    });
    
    let new_video_player_button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = ssb.clone();

    let color_picker = Rc::new(RefCell::new(ColorPool::new(["red", "blue", "green", "black", "coral", "lavender"].into_iter().map(String::from).collect())));
    
    let color_picker_clone = color_picker.clone();
    // Adds new video player and new columns to split table
    let split_table_clone = split_table.clone();
    let sync_manager_clone = sync_manager.clone();
    new_video_player_button.connect_clicked(move |_| {
        let count = *unsafe{ get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() };
        if count as u32 == MAX_VIDEO_PLAYERS {
            println!("Max video players reached");
            return;
        }
        
        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        
        // Sets up new video player
        let video_player_id = get_next_id().to_string();
        let new_player = VideoPlayer::new(video_player_id.as_str());
        let mut picker = color_picker_clone.borrow_mut();
        let color = picker.assign_color(video_player_id.as_str()).unwrap();
        new_player.set_color(color.as_str());
        new_player.setup_event_handlers();

        split_table_clone.add_empty_column(video_player_id.as_str());
        
        let split_table_clone_clone = split_table_clone.clone();
        // Listens to the split button from a video player
        // args[1] ID u32: index from the video player thats button was pressed
        // args[2] Position u64: time in nano seconds that the video player playback head was at when the button was pressed
        new_player.connect_local("split-button-clicked", false, move |args| {
            let video_player_id: String = args[1].get().unwrap();
            let video_player_position: u64 = args[2].get().unwrap();
            // Sets the time for the selected row
            if let Err(e) = split_table_clone_clone.set_split(video_player_id.as_str(), video_player_position) {
                eprintln!("{e}");
            }
            None
        });

        let split_table_clone_clone = split_table_clone.clone();
        new_player.connect_local("set-start-button-clicked", false, move |args| {
            let video_player_id: String = args[1].get().unwrap();
            let video_player_position: u64 = args[2].get().unwrap();
            if let Err(e) = split_table_clone_clone.set_start_time_offset(video_player_id.as_str(), video_player_position) {
                eprintln!("{e}");
            }
            None
        });

        new_player.connect_local("seek-bar-pressed", false, glib::clone!(
            #[strong(rename_to = shared_seek_bar)] shared_seek_bar_clone,
            move |_| {
                if shared_seek_bar.get_control_state() {
                    println!("User interacted with video player -> toggling control off");
                    shared_seek_bar.toggle_has_control();
                }
                None
            }
        ));

        new_player.connect_local("pipeline-built", false, glib::clone!(
            #[strong(rename_to = sync_man)] sync_manager_clone,
            #[strong(rename_to = pipeline_id)] video_player_id,
            #[strong(rename_to = video_player)] new_player,
            move |_| {
                let pipeline = video_player.pipeline();
                //split_table.reset_individual_video_segments(video_player_index);
                sync_man.add_pipeline(pipeline_id.as_str(), pipeline);
                None
            }
        ));

        new_player.connect_local("remove-video-player", false, glib::clone!(
            #[strong(rename_to = sync_man)] sync_manager_clone,
            #[strong(rename_to = pipeline_id)] video_player_id,
            #[strong(rename_to = video_player)] new_player,
            #[strong(rename_to = video_player_container)] video_container_clone,
            #[strong(rename_to = shared_seek_bar)] shared_seek_bar_clone,
            #[strong(rename_to = split_table)] split_table_clone,
            #[strong(rename_to = color_picker)] color_picker_clone,
            move |_| {
                println!("removing video player");
                split_table.remove_column(pipeline_id.as_str());
                shared_seek_bar.remove_marks(pipeline_id.as_str());
                sync_man.remove_pipeline(pipeline_id.as_str());
                let count = *unsafe{ get_data::<usize>(&video_player_container, "count").unwrap().as_ref() };
                store_data(&video_player_container, "count", count - 1);
                let number_of_columns = ((count as u32).saturating_sub(1)).clamp(1,3);
                video_player_container.set_max_children_per_line(number_of_columns);
                video_player_container.set_min_children_per_line(number_of_columns);
                let mut picker = color_picker.borrow_mut();
                picker.release_color(pipeline_id.as_str());
                if let Some(flowbox_child) = video_player.parent().and_then(|x| x.dynamic_cast::<FlowBoxChild>().ok()) {
                    video_player_container.remove(&flowbox_child);
                }
                video_player.cleanup();
                None
            }
        ));
        
        // Adds start time offset entry text to start_time_offset liststore/columnview
        let new_start_time_offset_time_entry = match split_table_clone.add_start_time_offset_row(video_player_id.as_str()) {
            Ok(te) => te,
            Err(e) => {
                panic!("{e}")
            }
        };
        
        new_start_time_offset_time_entry.connect_notify_local(Some("time"), glib::clone!(
            #[weak(rename_to = shared_seek_bar)] shared_seek_bar_clone,
            move |_, _| {
                shared_seek_bar.update_timeline_length();
            }
        ));
            
        // Adds two columns to split table for each new video player
        // Column 1: (Time) Split time -> time since the start of the clip
        // Column 2: (Duration) Segment time -> time since the last split
        split_table_clone.add_column(video_player_id.to_string().as_str(), video_player_id.as_str(), "relative-time");
        split_table_clone.add_column(video_player_id.to_string().as_str(), video_player_id.as_str(), "duration");

        // Updates formatting of the video players and adds the new video player to the container
        let number_of_columns = (count as u32 + 1).clamp(1,3);
        video_container_clone.set_max_children_per_line(number_of_columns);
        video_container_clone.set_min_children_per_line(number_of_columns);
        video_container_clone.append(&new_player);
        
        let video_player_index = count as u32;
        // Updates video_container data keeping track of the active video players
        store_data(&video_container_clone, "count", count + 1);

        
        split_table_clone.connect_column_to_seekbar(&video_container_clone, video_player_index);
        shared_seek_bar_clone.connect_column(video_player_id.as_str(), color.as_str());

        new_player.load_file(window);
    });

    let shared_seek_bar_clone = ssb.clone();
    toggle_shared_video_play.connect_clicked(move |_| {
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
                for child in flowbox_children(&video_container) {
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
                    video_player.unparent();
                    video_player.cleanup();
                }
                let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
                let sync_manager = unsafe { get_data::<SyncManager>(&window, "sync_manager").unwrap().as_ref() };
                unsafe {
                    sync_manager.run_dispose();
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