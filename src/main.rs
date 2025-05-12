mod video_pipeline;
use std::time::Duration;

use gio::ListStore;
use glib::{shared, timeout_add_local};
use glib::{random_int_range, ExitCode, prelude::ObjectExt, Regex, RegexCompileFlags, RegexMatchFlags};
use gstreamer::event::Seek;
use gstreamer::{Clock, ClockTime};
use gtk::subclass::fixed;
use gtk::{Adjustment, ColumnViewColumn, EventControllerFocus, FlowBox, FlowBoxChild, ListItem, SelectionMode, SingleSelection};
use gtk::{ gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button, ColumnView, CssProvider, Entry, Label};
use gstgtk4;
mod widgets;
use widgets::video_player_widget::seek_bar::{self, SeekBar};
use widgets::video_player_widget::video_player::{self, VideoPlayer};
use widgets::split_panel::splits::VideoSegment;
use widgets::split_panel::timeentry::TimeEntry;

const MAX_VIDEO_PLAYERS: u32 = 6;
const STARTING_TIME: u64 = 0;

#[derive(Clone)]
enum SegmentField {
    Time,
    Duration,
}

fn load_css(path: &str) {
    let provider = CssProvider::new();
    match std::env::current_dir() {
        Ok(current_dir) => {
            let file = gio::File::for_path(current_dir.join(path));
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

fn flowbox_children(flowbox: &FlowBox) -> impl Iterator<Item = gtk::Widget> {
    std::iter::successors(flowbox.first_child(), |w| w.next_sibling())
}

fn build_ui(app: &Application) -> Builder {
    let builder = Builder::from_resource("/mainwindow/mwindow.ui");
    let _column_builder = Builder::from_resource("/spanel/spanel.ui");

    load_css("src\\widgets\\main_window\\style.css");
    load_css("src\\widgets\\split_panel\\style.css");
    
    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
    let column_view_container: Box = builder.object("split_container").expect("Failed to column_view_container from UI File");
    let video_container: FlowBox = builder.object("video_container").expect("Failed to get video_container from UI File");
    let add_row_above_button: Button = builder.object("add_row_above_button").expect("Failed to get add_row_above_button from UI File");
    let add_row_below_button: Button = builder.object("add_row_below_button").expect("Failed to get add_row_below_button from UI File");
    let shared_seek_bar_container: Box = builder.object("shared_seek_bar_container").expect("Failed to get shared_seek_bar_container from UI File");
    
    let shared_previous_segment_button: Button = builder.object("shared_previous_segment_button").expect("Failed to get shared_previous_segment_button from UI File");
    let shared_previous_frame_button: Button = builder.object("shared_previous_frame_button").expect("Failed to get shared_previous_frame_button from UI File");
    let shared_play_button: Button = builder.object("shared_play_button").expect("Failed to get shared_play_button from UI File");
    let shared_next_frame_button: Button = builder.object("shared_next_frame_button").expect("Failed to get share_play_button from UI File");
    let shared_next_segment_button: Button = builder.object("shared_next_segment_button").expect("Failed to get shared_next_segment_button from UI File");
    let jump_to_segment_button: Button = builder.object("jump_to_segment_button").expect("Failed to get jump_to_segment_button from UI File");
    
    video_container.set_homogeneous(true);
    video_container.set_valign(gtk::Align::Fill);
    video_container.set_selection_mode(SelectionMode::None);
    video_container.set_column_spacing(0);
    
    let (model, column_view) = create_column_view();
    column_view_container.append(&column_view);

    let video_container_clone = video_container.clone();
    let column_view_clone = column_view.clone();
    shared_previous_segment_button.connect_clicked(move |_| {
        let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
        let selected_index = selection_model.selected();
        let previous_index = selected_index.saturating_sub(1);
        selection_model.set_selected(previous_index);
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
                if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                    let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
                    if let Ok(result) = pipeline.seek_position(time) {
                        println!("Shared pipeline seek for video player {video_player_index} to position {time}");
                    }
                }
            } else {
                eprintln!("No pipeline for index {video_player_index}");
            }
        }
        println!("Pressed shared preivous segment button");
    });

    let video_container_clone = video_container.clone();
    shared_previous_frame_button.connect_clicked(move |_| {
        let mut child_opt = video_container_clone.first_child();

        while let Some(child) = child_opt {
            if let Some(fb_child) = child.downcast_ref::<FlowBoxChild>() {
                if let Some(content) = fb_child.child() {
                    if let Some(video_player) = content.downcast_ref::<VideoPlayer>() {
                        let gstman_weak = video_player.pipeline();
                        if let Some(gstman) = gstman_weak.upgrade() {
                            if let Ok(mut guard) = gstman.lock() {
                                if let Some(ref mut pipeline) = *guard {
                                    pipeline.frame_backward();
                                    
                                } else {
                                    eprintln!("No Video Pipeline available");
                                }
                            } else {
                                eprintln!("Failed to aquire lock on Video pipeline");
                            }
                        }
                    }
                }
            }
            child_opt = child.next_sibling();
        }
        println!("Pressed shared previous frame button");
    });

    let video_container_clone = video_container.clone();
    shared_play_button.connect_clicked(move |_| {
        let mut child_opt = video_container_clone.first_child();

        while let Some(child) = child_opt {
            if let Some(fb_child) = child.downcast_ref::<FlowBoxChild>() {
                if let Some(content) = fb_child.child() {
                    if let Some(video_player) = content.downcast_ref::<VideoPlayer>() {
                        let gstman_weak = video_player.pipeline();
                        if let Some(gstman) = gstman_weak.upgrade() {
                            if let Ok(mut guard) = gstman.lock() {
                                if let Some(ref mut pipeline) = *guard {
                                    pipeline.play_video();
                                    
                                } else {
                                    eprintln!("No Video Pipeline available");
                                }
                            } else {
                                eprintln!("Failed to aquire lock on Video pipeline");
                            }
                        }
                    }
                }
            }
            child_opt = child.next_sibling();
        }
        println!("Pressed shared play button");
    });

    let video_container_clone = video_container.clone();
    shared_next_frame_button.connect_clicked(move |_| {
        let mut child_opt = video_container_clone.first_child();

        while let Some(child) = child_opt {
            if let Some(fb_child) = child.downcast_ref::<FlowBoxChild>() {
                if let Some(content) = fb_child.child() {
                    if let Some(video_player) = content.downcast_ref::<VideoPlayer>() {
                        let gstman_weak = video_player.pipeline();
                        if let Some(gstman) = gstman_weak.upgrade() {
                            if let Ok(mut guard) = gstman.lock() {
                                if let Some(ref mut pipeline) = *guard {
                                    pipeline.frame_forward();
                                    
                                } else {
                                    eprintln!("No Video Pipeline available");
                                }
                            } else {
                                eprintln!("Failed to aquire lock on Video pipeline");
                            }
                        }
                    }
                }
            }
            child_opt = child.next_sibling();
        }
        println!("Pressed shared next frame button");
    });

    let video_container_clone = video_container.clone();
    let column_view_clone = column_view.clone();
    shared_next_segment_button.connect_clicked(move |_| {
        let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
        let selected_index = selection_model.selected();
        let next_index = (selected_index + 1).clamp(0, selection_model.n_items() - 1);
        selection_model.set_selected(next_index);
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
                if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                    let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
                    if let Ok(result) = pipeline.seek_position(time) {
                        println!("Shared pipeline seek for video player {video_player_index} to position {time}");
                    }
                }
            } else {
                eprintln!("No pipeline for index {video_player_index}");
            }
        }
        println!("Pressed shared next segment button");
    });

    let video_container_clone = video_container.clone();
    let column_view_clone = column_view.clone();
    jump_to_segment_button.connect_clicked(move |_| {
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
                let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
                if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                    let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
                    if let Ok(result) = pipeline.seek_position(time) {
                        println!("Shared pipeline seek for video player {video_player_index} to position {time}");
                    }
                }
            } else {
                eprintln!("No pipeline for index {video_player_index}");
            }
        }
        println!("Pressed jump to segment button");
    });


    
    let shared_seek_bar = SeekBar::new(0, true);
    shared_seek_bar.add_tick_callback_timeout();
    shared_seek_bar.set_can_target(false);
    shared_seek_bar.set_can_focus(false);
    shared_seek_bar_container.append(&shared_seek_bar);

    // Adds first row of segment names to the split table
    let column_view_clone = column_view.clone();
    add_name_column(&column_view_clone, "Segment Name");

    // Add data to video_container to keep track of the number of active videos
    let initial_child_count = 0_usize;
    store_data(&video_container, "count", initial_child_count);

    // Adds an initial row
    add_empty_row_with_columns(&model, MAX_VIDEO_PLAYERS);
    
    
    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = shared_seek_bar.clone();
    add_row_above_button.connect_clicked(move |_| {
        let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
        if let Some(_selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            let selected_index = selection_model.selected();
            insert_empty_row(&model_clone, selected_index, MAX_VIDEO_PLAYERS);
            connect_row_to_seekbar(&model_clone, &video_container_clone, selected_index);
            let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
            connect_row_to_shared_seekbar(&model_clone, &shared_seek_bar_clone, selected_index, video_player_count);
        }
    });

    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = shared_seek_bar.clone();
    add_row_below_button.connect_clicked(move |_| {
        let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
        if let Some(_selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            let selected_index = selection_model.selected();
            insert_empty_row(&model_clone, selected_index + 1, MAX_VIDEO_PLAYERS);
            connect_row_to_seekbar(&model_clone, &video_container_clone, selected_index + 1);
            let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
            connect_row_to_shared_seekbar(&model_clone, &shared_seek_bar_clone, selected_index + 1, video_player_count);
        }
    });



    let button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let column_view_clone = column_view.clone();
    let model_clone = model.clone();
    let video_container_clone = video_container.clone();
    
    
    let shared_seek_bar_clone = shared_seek_bar.clone();
    
    // Adds new video player and new columns to split table
    button.connect_clicked(move |_| {
        let count = *unsafe{ get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() };
        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        
        // Sets up new video player
        let new_player = VideoPlayer::new(count as u32);
        new_player.setup_event_handlers(window);
        
        let model_clone_clone = model_clone.clone();
        let column_view_clone_clone = column_view_clone.clone();
        
        
        //let shared_seek_bar_clone_clone = shared_seek_bar_clone.clone();
        
        
        // Listens to the split button from a video player
        // args[1] ID u32: index from the video player thats button was pressed
        // args[2] Position u64: time in nano seconds that the video player playback head was at when the button was pressed
        new_player.connect_local("button-clicked", false, move |args| {
            let video_player_index: u32 = args[1].get().unwrap();
            let video_player_position: u64 = args[2].get().unwrap();
            // Sets the time for the selected row
            let selection_model = column_view_clone_clone.model().and_downcast::<SingleSelection>().unwrap();
            if let Some(selected_segment) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                let selected_index = selection_model.selected();
                selected_segment.set_time(video_player_index as usize, video_player_position);
                
                // update shared_seek_bar timeline length to be at the latest split
                // fixes any conflicts resulting from adding the new time
                correct_conflicts(&model_clone_clone, video_player_index, selected_index); // this updates durations after fixing any conflicts
                //update_durations(&model_clone_clone, video_player_index, selected_index);
            } else {
                eprintln!("No segment selected");
            }
            None
        });

        let shared_seek_bar_clone_clone = shared_seek_bar_clone.clone();
        new_player.connect_local("timeline-length-acquired",false, move |args| {
            let timeline_length: u64 = args[1].get().unwrap();
            let current_timeline_length = shared_seek_bar_clone_clone.get_timeline_length();
            if timeline_length > current_timeline_length {
                
                //shared_seek_bar_clone_clone.set_timeline_length(timeline_length);
            }
            None
        });
        
        // Adds two columns to split table for each new video player
        // Column 1: (Time) Split time -> time since the start of the clip
        // Column 2: (Duration) Segment time -> time since the last split
        let name = random_int_range(0, 99);
        let model_clone_clone = model_clone.clone();
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), count, &format!("time-{}", count));
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), count, &format!("duration-{}", count));

        
        // Updates formatting of the video players and adds the new video player to the container
        let number_of_columns = (count as u32 + 1).clamp(1,3);
        video_container_clone.set_max_children_per_line(number_of_columns);
        video_container_clone.set_min_children_per_line(number_of_columns);
        video_container_clone.append(&new_player);
        
        let video_player_index = count as u32;
        // Updates video_container data keeping track of the active video players
        store_data(&video_container_clone, "count", count + 1);

        connect_column_to_seekbar(&model_clone_clone, &video_container_clone, video_player_index);
        let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
        connect_column_to_shared_seekbar(&model_clone, &shared_seek_bar_clone, video_player_index, video_player_count);
    });

    // Debug function to print the split data in liststore
    // Used to make sure the split data is correctly being stored as this is separate from the displayed information in the table
    // let button: Button = builder.object("print_splits_button").expect("Failed to get new split button");
    // let model_clone = model.clone();
    // button.connect_clicked(move |_| {
    //     print_vec(&model_clone);
    // });

    app.add_window(&window);
    window.show();
    builder
}

// Updates durations to correctly match the set segment times
fn update_durations(model: &ListStore, video_player_index: u32, starting_row_index: u32) {
    let number_of_rows = model.n_items();
    let mut previous_time: u64 = match get_previous_time(model, video_player_index, starting_row_index) {
        Some(time) => time,
        None => STARTING_TIME,
    };
    for i in starting_row_index..number_of_rows {
        let current_video_segment = model.item(i).and_downcast::<VideoSegment>().unwrap();
        let current_time = current_video_segment.get_time(video_player_index as usize);
        let current_duration = current_video_segment.get_duration(video_player_index as usize);
        if let (Some(time), Some(_duration)) = (current_time, current_duration) {
            if time == u64::MAX {
                continue;
            }
            current_video_segment.set_duration(video_player_index as usize, time - previous_time);
            previous_time = time;
        }
    }
}

// Gets most recent previous time from the given row index
fn get_previous_time(model: &ListStore, video_player_index: u32, row_index: u32) -> Option<u64> {
    for i in (0..row_index).rev() {
        let item = model.item(i).and_downcast::<VideoSegment>().unwrap();
        if let Some(time) = item.get_time(video_player_index as usize) {
            if time != u64::MAX {
                return Some(time);
            }
        }
    }
    return None;
}

// Updates the times from a starting row to match the durations
fn update_times(model: &ListStore, video_player_index: u32, starting_row_index: u32) {
    let number_of_rows = model.n_items();
    let mut previous_time = match get_previous_time(model, video_player_index, starting_row_index) {
        Some(time) => time,
        None => 0,
    };
    for i in starting_row_index..number_of_rows {
        let current_video_segment = model.item(i).and_downcast::<VideoSegment>().unwrap();
        let current_time = current_video_segment.get_time(video_player_index as usize);
        let current_duration = current_video_segment.get_duration(video_player_index as usize);
        if let (Some(_time), Some(duration)) = (current_time, current_duration) {
            let new_time = previous_time + duration;
            current_video_segment.set_time(video_player_index as usize, new_time);
            previous_time = new_time;
        }

    }
}

// Ensures the video segments hold a non-decreasing order around the starting_row_index row
// Reduces the times that come before the starting row to match starting_row_time if they are greater
// Increase the times that come after the starting row to match starting_row_time if they are smaller
// Calls update_durations after fixing any conflicts
fn correct_conflicts(model: &ListStore, video_player_index: u32, starting_row_index: u32) {
    let starting_row = model.item(starting_row_index).and_downcast::<VideoSegment>().unwrap();
    let starting_row_time = starting_row.get_time(video_player_index as usize).unwrap();
    for i in (0..starting_row_index).rev() {
        let current_row = model.item(i).and_downcast::<VideoSegment>().unwrap();
        match current_row.get_time(video_player_index as usize) {
            Some(time) => {
                if time == u64::MAX {
                    continue;
                }
                if time > starting_row_time {
                    current_row.set_time(video_player_index as usize, starting_row_time);
                } else {
                    break;
                }
            }
            None => { }
        }
    }
    for i in starting_row_index+1..model.n_items() {
        let current_row = model.item(i).and_downcast::<VideoSegment>().unwrap();
        match current_row.get_time(video_player_index as usize) {
            Some(time) => {
                if time == u64::MAX {
                    continue;
                }
                if time < starting_row_time {
                    current_row.set_time(video_player_index as usize, starting_row_time);
                } else {
                    break;
                }
            }
            None => { }
        }
    }
    update_durations(model, video_player_index, 0);
}


// Used to make sure the split data is correctly being stored as this is separate from the displayed information in the table
fn print_vec(model: &ListStore) {
    println!("Splits");
    for i in 0..model.n_items() {
        print!("Row: {i} ");
        if let Some(item) = model.item(i).and_downcast::<VideoSegment>() {
            for j in 0..item.get_segment_count() {
                let time = item.get_time(j).unwrap();
                let duration = item.get_duration(j).unwrap();
                print!("{time}, {duration} |");
            }
        }
        println!("");
    }
}

// Converts a GStreamer ClockTime to a String
// Format: MM:SS.sss or HH:MM:SS.sss if hours exist
fn format_clock(time: u64) -> String {
    if time == u64::MAX {
        return String::new();
    }
    let mut ret = ClockTime::from_nseconds(time).to_string();
    let hours_offset = ret.find(":").unwrap();
    let hour= ret[..hours_offset].to_string();
    let hour_parsed: u32 = hour.parse().unwrap();
    if hour_parsed == 0 {
        ret.drain(..hours_offset+1);
    }
    let split = ret.find(".").unwrap();
    let digits_after_decimal_point = 3;
    ret.truncate(split + digits_after_decimal_point + 1);
    ret
}

// Converts time from String type formatted as MM:SS.sss to nanoseconds
fn string_to_nseconds(time: &String) -> Option<u64> {
    let (min, rest) = time.split_once(":").unwrap();
    let (sec, subseconds) = rest.split_once(".").unwrap();

    let minutes = min.parse::<u64>().unwrap();
    let seconds = sec.parse::<u64>().unwrap();

    let nanos = match subseconds.len() {
        0 => 0,
        1 => subseconds.parse::<u64>().unwrap() * 100_000_000, // 0.1s = 100_000_000ns
        2 => subseconds.parse::<u64>().unwrap() * 10_000_000,  // 0.01s = 10_000_000ns
        3 => subseconds.parse::<u64>().unwrap() * 1_000_000,   // 0.001s = 1_000_000ns
        4 => subseconds.parse::<u64>().unwrap() * 100_000,     // 0.0001s
        5 => subseconds.parse::<u64>().unwrap() * 10_000,      // ...
        6 => subseconds.parse::<u64>().unwrap() * 1_000,
        7 => subseconds.parse::<u64>().unwrap() * 100,
        8 => subseconds.parse::<u64>().unwrap() * 10,
        _ => subseconds.parse::<u64>().unwrap() // assume already in nanoseconds
    };
    let total_nanos = minutes * 60 * 1_000_000_000 + seconds * 1_000_000_000 + nanos;
    return Some(total_nanos);
}

fn create_column_view() -> (ListStore, ColumnView) {
    // Create a ListStore to hold VideoSegment data
    let model = gio::ListStore::new::<VideoSegment>();
    let model_clone = model.clone();

    let selection_model = gtk::SingleSelection::new(Some(model_clone));
    
    // Create the ColumnView
    let column_view = gtk::ColumnView::new(Some(selection_model));
    
    (model, column_view)
}

// Add new column to the column view
// index: video player id NOT the column index 
// field: Time or Duration
// Each video player gets two columns one for time and one for duration
fn add_column(column_view: &gtk::ColumnView, _model: &ListStore, title: &str, video_player_index: usize, prop_name: &str) {
    let factory = gtk::SignalListItemFactory::new();
    // Creates the entry objects
    factory.connect_setup(move |_, list_item| {
        let entry = gtk::Entry::new();
        list_item.set_child(Some(&entry));
    });

    // Binds the stored data to the displayed entry objects
    let model_clone = _model.clone();
    let property = prop_name.to_string();
    
    factory.connect_bind(move |_, list_item| {
        let item = list_item.item().and_then(|obj| obj.downcast::<VideoSegment>().ok()).expect("The item is not a VideoSegment");
        let entry = list_item.child().and_then(|child| child.downcast::<Entry>().ok()).expect("The child widget is not Entry");
        let binding = item
            .bind_property(&property, &entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, value: u64| { // Converts the u64 time to formatted MM:SS.sss time to display
                Some(format_clock(value).to_value())
            })
            .build();
        store_data(list_item, &format!("binding-{}", property), binding); 

        entry.connect_activate(glib::clone!(
            #[strong] property,
            #[weak(rename_to = entry)] entry,
            #[weak(rename_to = video_segment)] item,
            #[weak(rename_to = model)] model_clone,
            move |_| {
                match &property {
                    prop if prop.starts_with("name") => {
                        // do name stuff here
                        println!("Change name not impletemented");
                    }
                    prop if prop.starts_with("time-") => {
                        println!("Changing {}", property);
                        let row_index = model.find(&video_segment).unwrap();
                        let valid_entry = validate_split_table_entry(&entry);
                        if !valid_entry { // Restores segment data if invalid entry
                            let stored_entry_data = video_segment.property(property.as_str());
                            entry.set_text(format_clock(stored_entry_data).as_str());
                        } else { // updates segment data with new entry and fixes any conflicts
                            let new_time = string_to_nseconds(&entry.text().to_string()).unwrap();
                            video_segment.set_time(video_player_index, new_time);
                            correct_conflicts(&model, video_player_index as u32, row_index);
                        }
                    }
                    prop if prop.starts_with("duration-") => {
                        println!("Changing {}", property);
                        let row_index = model.find(&video_segment).unwrap();
                        let valid_entry = validate_split_table_entry(&entry);
                        if !valid_entry {// Restores segment data if invalid entry
                            let stored_entry_data = video_segment.property(property.as_str());
                            entry.set_text(format_clock(stored_entry_data).as_str());
                        } else { // updates segment data with new entry and fixes any conflicts
                            let new_duration = string_to_nseconds(&entry.text().to_string()).unwrap();
                            video_segment.set_duration(video_player_index, new_duration);
                            let previous_time: u64 = match get_previous_time(&model, video_player_index as u32, row_index) {
                                Some(time) => time,
                                None => 0,
                            };
                            video_segment.set_time(video_player_index, previous_time + new_duration);
                            correct_conflicts(&model, video_player_index as u32, row_index);
                        }
                    }
                    _ => {
                        eprintln!("Invalid property: {}", property);
                    }
                }
            }
        ));
    });

    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column_view.append_column(&column);
}

// Validates the formatting of user input into the split table
fn validate_split_table_entry(entry: &Entry) -> bool {
    let input = entry.text().to_string();
    let pattern = r"^[0-5][0-9]:[0-5][0-9]\.\d{3}$";
    // Checks if the input matches the format: MM:SS.sss
    let re = Regex::match_simple(pattern, input.clone(), RegexCompileFlags::empty(), RegexMatchFlags::empty());
    if !re {
        println!("Entry is not in valid format");
    }
    re
}

// Adds column for segment names
fn add_name_column(column_view: &gtk::ColumnView, title: &str) {
    let factory = gtk::SignalListItemFactory::new();
    
    // Creates the entry objects
    factory.connect_setup(|_factory, list_item: &ListItem| {
        let entry = Entry::new();
        list_item.set_child(Some(&entry));
    });
    
    // Binds the stored data to the displayed entry objects
    factory.connect_bind(|_factory, list_item: &ListItem| {
        let entry = list_item.child().unwrap().downcast::<Entry>().expect("The child is not an Entry");
        let item = list_item.item();
        let video_segment = item.and_downcast_ref::<VideoSegment>().expect("Item is not a VideoSegment");
        let current_name = video_segment.get_name();
        entry.set_text(&current_name);
        
        // Updates segment name from user input
        entry.connect_changed(glib::clone!(
            #[weak(rename_to = seg)] video_segment,
            move |entry| {
                let new_name = entry.text().to_string();
                seg.set_name(new_name);
            } 
        ));
    });
    
    // Adds the new column to column view
    let new_column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column_view.append_column(&new_column);
    
}

fn remove_column(column_view: &gtk::ColumnView) {
    
    let columns = column_view.columns();
    if let Some(last_column) = columns.into_iter().last() {
        let x = last_column.unwrap().downcast::<ColumnViewColumn>().unwrap();
        column_view.remove_column(&x);
    }
}

// Adds empty row to the liststore at the specified index
fn insert_empty_row(model: &ListStore, insert_index: u32, number_of_columns: u32) {
    let seg = VideoSegment::new(insert_index.to_string().as_str());
    for _ in 0..number_of_columns {
        seg.add_empty_segment();
    }
    model.insert(insert_index, &seg);
}

// Adds empty row to end of liststore
fn add_empty_row_with_columns(model: &ListStore, number_of_columns: u32) {
    let row_count = model.n_items();
    insert_empty_row(model, row_count, number_of_columns);
}

// Called right after adding a new row
// Connects the new row of times in the split table to marks on the seekbar for each video
fn connect_row_to_seekbar(model: &ListStore, video_container: &FlowBox, row_index: u32) {
    let video_player_count = *unsafe { get_data::<usize>(video_container, "count").unwrap().as_ref() } as i32; 
    let row = model.item(row_index).and_downcast::<VideoSegment>().unwrap();
    let row_count = model.n_items();
    for i in 0..video_player_count {
        let video_player = video_container.child_at_index(i)
            .and_then(|child| child.child())
            .and_downcast::<VideoPlayer>()
            .unwrap();
        let time = row.get_time_entry_copy(i as usize);
        // id should always be row_count regardless of if the row is inserted in the middle.
        // not sure if it will matter but this should give marks unique ids
        let row_id = row_count - 1; 
        video_player.connect_time_to_seekbar(format!("video-{i}, row-{row_id}"), time, "black");
    }
}

// Called right after adding a new video player
// Connects each of the times assoiciated with the new video to the seekbar
fn connect_column_to_seekbar(model: &ListStore, video_container: &FlowBox, column_index: u32) {
    let row_count = model.n_items();
    let video_player = video_container.child_at_index(column_index as i32)
        .and_then(|child| child.child())
        .and_downcast::<VideoPlayer>()
        .unwrap();
    for i in 0..row_count {
        let row = model.item(i).and_downcast::<VideoSegment>().unwrap();
        let time = row.get_time_entry_copy(column_index as usize);
        // id are given in order as they have already been created
        video_player.connect_time_to_seekbar(format!("video-{column_index}, seg-{i}"), time, "black");
    }
}

fn connect_row_to_shared_seekbar(model: &ListStore, seekbar: &SeekBar, row_index: u32, video_player_count: i32) {
    let row = model.item(row_index).and_downcast::<VideoSegment>().unwrap();
    let row_count = model.n_items();
    let colors = vec!["red", "blue", "green", "black", "coral", "lavender"];
    for i in 0..video_player_count {
        let time = row.get_time_entry_copy(i as usize);
        // id should always be row_count regardless of if the row is inserted in the middle.
        // not sure if it will matter but this should give marks unique ids
        let row_id = row_count - 1; 
        seekbar.add_mark(format!("video-{i}, row-{row_id}"), time, colors[i as usize]);
    }
}

fn connect_column_to_shared_seekbar(model: &ListStore, seekbar: &SeekBar, column_index: u32, video_player_count: i32) {
    let row_count = model.n_items();
    let colors = vec!["red", "blue", "green", "black", "coral", "lavender"];
    for i in 0..row_count {
        let row = model.item(i).and_downcast::<VideoSegment>().unwrap();
        let time = row.get_time_entry_copy(column_index as usize);
        // id are given in order as they have already been created
        seekbar.add_mark(format!("video-{column_index}, seg-{i}"), time, colors[(video_player_count - 1) as usize]);
    }
}

fn remove_row(model: &gio::ListStore, row_index: u32) {
    if model.n_items() == 0 {
        eprintln!("No row to remove");
        return;
    }
    if row_index >= model.n_items() {
        eprintln!("No selected row");
        return;
    }
    
    model.remove(row_index);
}

// Applys data to a widget given a key and value pair
fn store_data<T: 'static>(widget: &impl ObjectExt, key: &str, value: T) {
    unsafe {
        widget.set_data(key, value);
    }
}

// Retrieves data from a widget given a key
fn get_data<T: 'static>(widget: &impl ObjectExt, key: &str) -> Option<std::ptr::NonNull<T>> {
    unsafe { widget.data::<T>(key) }
}

fn main() -> glib::ExitCode {
    let run_app = 0;
    if run_app == 0 {
        gstreamer::init().unwrap();
        gtk::init().unwrap();

        std::env::set_var("GTK_THEME", "Adwaita:dark");

        gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

        gio::resources_register_include!("vplayer.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("mwindow.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("spanel.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("seekbar.gresource")
            .expect("Failed to register resources.");
        
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
            .expect("Failed to register resources.");

        gio::resources_register_include!("mwindow.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("spanel.gresource")
            .expect("Failed to register resources.");
        
        let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);


        app.connect_activate(|app| {

            let window = ApplicationWindow::new(app);

            window.set_default_size(800, 600);
            window.set_title(Some("Video Player"));

            load_css("src\\widgets\\main_window\\style.css");

            let main_box = Box::new(gtk::Orientation::Horizontal, 10);
            // let (liststore, column_view) = test_create();
            
            // liststore.insert(0, &VideoSegment::new("Hi"));
            // liststore.insert(1, &VideoSegment::new("Woooo"));
            
            // for i in 0..2 {
            //     let segment = liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            //     for _ in 0..3 {
            //         segment.add_empty_test_segment();
            //     }
            // }

            // test_column(&column_view, "Name", "name");
            // for i in 0..6 {
            //     test_column(&column_view, &format!("Time {}", i), &format!("time-{}", i));
            //     test_column(&column_view, &format!("Duration {}", i), &format!("duration-{}", i));
            // }
            
            

            // let button = Button::new();
            // button.set_label("edit random cell");

            // let store_clone = liststore.clone();
            // button.connect_clicked(move |_| {
            //     let row = random_int_range(0, 2);
            //     let column = random_int_range(1, 13);
            //     let value = random_int_range(0, i32::MAX) as u64;
            //     let segment = store_clone.item(row as u32).and_downcast::<VideoSegment>().unwrap();
            //     let prop_name = match (column - 1) % 2 {
            //         0 => {
            //             let n = (column - 1) / 2;
            //             format!("time-{}", n)
            //         }
            //         1 => {
            //             let n = (column - 1) / 2;
            //             format!("duration-{}", n)
            //         }
            //         _ => unimplemented!()
            //     };
            //     segment.set_property(&prop_name, value);
            //     println!("Changed Row: {row}, Column: {column}, to {value}");
            // });

            // main_box.append(&column_view);
            // main_box.append(&button);

            window.set_child(Some(&main_box));
            window.show();
        });


        let res = app.run();

        unsafe {
            gstreamer::deinit();
        }
        return res
    } else if run_app == 2 {
        gstreamer::init().unwrap();
        gtk::init().unwrap();

        std::env::set_var("GTK_THEME", "Adwaita:dark");

        gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");

        gio::resources_register_include!("vplayer.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("mwindow.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("spanel.gresource")
            .expect("Failed to register resources.");

        gio::resources_register_include!("seekbar.gresource")
            .expect("Failed to register resources.");
        
        let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);
        app.connect_activate(move |app| {
            
            let window = ApplicationWindow::new(app);
            
            window.set_default_size(800, 600);
            window.set_title(Some("Video Player"));
            
            load_css("src\\widgets\\main_window\\style.css");
            
            let main_box = Box::new(gtk::Orientation::Horizontal, 10);
            let timeline_length = 1000000 as u64;
            let seekbar = SeekBar::new(timeline_length, false);
            let time = TimeEntry::new(0);
            let time_rc = std::rc::Rc::new(time);
            let time_rc_clone = time_rc.clone();
            seekbar.add_mark("1".to_string(), time_rc_clone, "black");

            let time_rc_clone = time_rc.clone();
            timeout_add_local(Duration::from_secs(1), move || {
                let time_w = time_rc_clone.get_time();
                time_rc_clone.set_time(time_w + 100000);
                glib::ControlFlow::Continue
            });
            // let new_box = Box::new(gtk::Orientation::Horizontal, 10);
            // let overlay_object = gtk::Overlay::new();
            // let fixed_object = gtk::Fixed::new();
            // fixed_object.set_receives_default(false);
            // fixed_object.set_can_target(false);
            // overlay_object.set_hexpand(true);
            
            // let scale = gtk::Scale::new(gtk::Orientation::Horizontal, Some(&Adjustment::new(0.0, 1.0, 100.0, 1.0, 0.0, 0.0)));
            // scale.set_hexpand(true);

            // overlay_object.add_overlay(&scale);
            // overlay_object.add_overlay(&fixed_object);

            // let mark = gtk::Label::new(Some("^"));

            // fixed_object.put(&mark, 0.0, 300.0);

            // new_box.append(&overlay_object);
            main_box.append(&seekbar);


            // //main_box.set_halign(gtk::Align::Fill);
            // let length: u64 = 10000000000;
            // let width = main_box.width();
            // let height = 50;
            // let sb = SeekBar::new(length, width, height);
            // main_box.append(&sb);

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