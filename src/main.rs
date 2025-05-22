mod video_pipeline;
use gio::ListStore;
use glib::{timeout_add_local};
use glib::{random_int_range, ExitCode, prelude::ObjectExt, Regex, RegexCompileFlags, RegexMatchFlags};
use gstreamer::{Clock, ClockTime};
use gtk::subclass::fixed;
use gtk::{Adjustment, ColumnViewColumn, EventControllerFocus, FlowBox, FlowBoxChild, ListItem, SelectionMode, SingleSelection, Window};
use gtk::{ gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button, ColumnView, CssProvider, Entry, Label};
use gstgtk4;
mod widgets;
use widgets::seek_bar::seek_bar::{self, SeekBar};
use widgets::seek_bar::shared_seek_bar::SharedSeekBar;
use widgets::video_player_widget::video_player::{self, VideoPlayer};
use widgets::split_panel::splits::VideoSegment;
use widgets::split_panel::timeentry::TimeEntry;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use gtk::prelude::GtkWindowExt;

const MAX_VIDEO_PLAYERS: u32 = 6;

#[derive(Clone)]
enum TimeDisplayMode {
    Absolute,
    Relative,
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
    let bottom_vbox: Box = builder.object("bottom_vbox").expect("Failed to get bottom_vbox from UI File");
    let start_time_offset_container: Box = builder.object("start_time_offset_container").expect("Failed to get start_time_offset_container from UI File");
    
    video_container.set_homogeneous(true);
    video_container.set_valign(gtk::Align::Fill);
    video_container.set_selection_mode(SelectionMode::None);
    video_container.set_column_spacing(0);
    
    let (model, column_view) = create_column_view::<VideoSegment>();
    column_view.set_reorderable(false);
    column_view.set_show_column_separators(true);
    column_view.set_show_row_separators(true);
    column_view.add_css_class("data-table");
    column_view_container.append(&column_view);

    let (start_time_offset_model, start_time_offset_column_view) = create_column_view::<TimeEntry>();
    start_time_offset_container.append(&start_time_offset_column_view);

    let start_time_offset_column_view_clone = start_time_offset_column_view.clone();
    let start_time_offset_model_clone = start_time_offset_model.clone();
    let split_table_model_clone = model.clone();
    build_start_time_offset_column(&start_time_offset_column_view_clone,
        &start_time_offset_model_clone, 
        &split_table_model_clone,
        "Start Time Offsets");
    

    let ssb = SharedSeekBar::new(&video_container, &column_view, &start_time_offset_model, &model);
    bottom_vbox.append(&ssb);
    
    // Adds an initial row
    add_empty_row_with_columns(&model, MAX_VIDEO_PLAYERS);
    
    // Adds first row of segment names to the split table
    let column_view_clone = column_view.clone();
    add_name_column(&column_view_clone, "Segment Name");

    // Add data to video_container to keep track of the number of active videos
    let initial_child_count = 0_usize;
    store_data(&video_container, "count", initial_child_count);
    
    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = ssb.clone();
    add_row_above_button.connect_clicked(move |_| {
        let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
        if let Some(_selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            let selected_index = selection_model.selected();
            insert_empty_row(&model_clone, selected_index, MAX_VIDEO_PLAYERS);
            connect_row_to_seekbar(&model_clone, &video_container_clone, selected_index);
            let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
            shared_seek_bar_clone.connect_row(selected_index, video_player_count as u32);
        }
    });

    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    let video_container_clone = video_container.clone();
    let shared_seek_bar_clone = ssb.clone();
    add_row_below_button.connect_clicked(move |_| {
        let selection_model = column_view_clone.model().and_downcast::<SingleSelection>().unwrap();
        if let Some(_selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            let selected_index = selection_model.selected();
            insert_empty_row(&model_clone, selected_index + 1, MAX_VIDEO_PLAYERS);
            connect_row_to_seekbar(&model_clone, &video_container_clone, selected_index + 1);
            let video_player_count = *unsafe { get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() } as i32;
            shared_seek_bar_clone.connect_row(selected_index + 1, video_player_count as u32);
        }
    });



    let new_video_player_button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let column_view_clone = column_view.clone();
    let model_clone = model.clone();
    let video_container_clone = video_container.clone();
    let start_time_offset_model_clone = start_time_offset_model.clone();
    
    //let shared_seek_bar_clone = shared_seek_bar.clone();
    let shared_seek_bar_clone = ssb.clone();
    
    // Adds new video player and new columns to split table
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
        
        let model_clone_clone = model_clone.clone();
        let column_view_clone_clone = column_view_clone.clone();
        // Listens to the split button from a video player
        // args[1] ID u32: index from the video player thats button was pressed
        // args[2] Position u64: time in nano seconds that the video player playback head was at when the button was pressed
        new_player.connect_local("split-button-clicked", false, move |args| {
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

        let start_time_offset_model_clone_clone = start_time_offset_model_clone.clone();
        let model_clone_clone = model_clone.clone();
        new_player.connect_local("set-start-button-clicked", false, move |args| {
            let video_player_index: u32 = args[1].get().unwrap();
            let video_player_position: u64 = args[2].get().unwrap();
            let time_entry = start_time_offset_model_clone_clone.item(video_player_index).and_downcast::<TimeEntry>().unwrap();
            time_entry.set_time(video_player_position);

            for i in 0..model_clone_clone.n_items() {
                let video_segment = model_clone_clone.item(i).and_downcast::<VideoSegment>().unwrap();
                if i == 0 {
                    let time = video_segment.get_time(video_player_index as usize).unwrap();
                    video_segment.set_duration(video_player_index as usize, time.saturating_sub(video_player_position));
                }

                video_segment.set_offset(video_player_index as usize, video_player_position);
            }
            None
        });

        // Adds start time offset entry text to start_time_offset liststore/columnview
        let new_start_time_offset_time_entry = TimeEntry::new(0);
        start_time_offset_model_clone.append(&new_start_time_offset_time_entry);
        
        // Adds two columns to split table for each new video player
        // Column 1: (Time) Split time -> time since the start of the clip
        // Column 2: (Duration) Segment time -> time since the last split
        let name = random_int_range(0, 99);
        let model_clone_clone = model_clone.clone();
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), count, &format!("relative-time-{}", count));
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), count, &format!("duration-{}", count));

        new_start_time_offset_time_entry.connect_notify_local(Some("time"), glib::clone!(
            #[weak(rename_to = shared_seek_bar)] shared_seek_bar_clone,
            move |_, _| {
                shared_seek_bar.update_timeline_length();
        }));



        
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
        shared_seek_bar_clone.connect_column(video_player_index, video_player_count as u32);
    });

    app.add_window(&window);
    window.show();
    builder
}

// Updates durations to correctly match the set segment times
fn update_durations(model: &ListStore, video_player_index: u32, starting_row_index: u32) {
    let number_of_rows = model.n_items();
    if number_of_rows == 0 {
        return;
    }

    let mut previous_time: u64 = match get_previous_time(model, video_player_index, starting_row_index) {
        Some(time) => time,
        None => {
            let video_segment = model.item(0).and_downcast::<VideoSegment>().unwrap();
            video_segment.get_offset(video_player_index as usize)
        },
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

fn create_column_view<T: 'static + IsA<glib::Object>>() -> (ListStore, ColumnView) {
    // Create a ListStore to hold VideoSegment data
    let model = gio::ListStore::new::<T>();
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
    
    let model_clone = _model.clone();
    let property = prop_name.to_string();
    let column_view_clone = column_view.clone();
    // Creates the entry objects
    factory.connect_setup(move |_, list_item| {
        let entry = gtk::Entry::new();
        entry.add_css_class("flat");
        entry.set_hexpand(true);
        entry.set_halign(gtk::Align::Fill);
        // When user enters new time into an entry the corresponding values in the videosegment will be updated
        // Any affected values are also updated
        entry.connect_activate(glib::clone!(
            #[strong] property,
            #[weak(rename_to = entry)] entry,
            #[weak(rename_to = list_item)] list_item,
            #[weak(rename_to = model)] model_clone,
            move |_| {
                if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                    match &property {
                        prop if prop.starts_with("name") => {
                            // do name stuff here
                            println!("Change name not impletemented");
                        }
                        prop if prop.starts_with("relative-time-") => {
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
            }
        ));
    
        entry.connect_has_focus_notify(glib::clone!(
            #[weak(rename_to = model)] model_clone,
            #[weak(rename_to = column_view)] column_view_clone,
            #[weak(rename_to = list_item)] list_item,
            move |_| {
                if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                    let row_index = model.find(&video_segment).unwrap_or(u32::MAX);
                    if row_index != u32::MAX {
                        let selection_model = column_view.model().and_downcast::<SingleSelection>().unwrap();
                        selection_model.select_item(row_index, true);
                    }
                }
            }
        ));
        list_item.set_child(Some(&entry));
    });

    // Binds the stored data to the displayed entry objects
    let property = prop_name.to_string();
    factory.connect_bind(move |_, list_item| {
        let item = list_item.item().and_then(|obj| obj.downcast::<VideoSegment>().ok()).expect("The item is not a VideoSegment");
        let entry = list_item.child().and_then(|child| child.downcast::<Entry>().ok()).expect("The child widget is not Entry");
        // Binds the u64 stored in the video segment to the entries formatted clock
        // Any changes to the videosegment will be updated in the entry object
        let binding = item.bind_property(&property, &entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .transform_to(move |_, value: u64| { 
                Some(format_clock(value).to_value())
            })
            .build();
        store_data(list_item, &format!("binding-{}", property), binding); 
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
    let column_view_clone = column_view.clone();
    // Creates the entry objects
    factory.connect_setup(move |_factory, list_item: &ListItem| {
        let entry = Entry::new();
        entry.add_css_class("flat");
        entry.set_hexpand(true);
        entry.set_halign(gtk::Align::Fill);
        
        // Updates segment name from user input
        entry.connect_activate(glib::clone!(
            #[weak(rename_to = list_item)] list_item,
            move |entry| {
                if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                    let new_name = entry.text().to_string();
                    video_segment.set_name(new_name);
                }
            } 
        ));
    
        entry.connect_has_focus_notify(glib::clone!(
            #[weak(rename_to = column_view)] column_view_clone,
            #[weak(rename_to = list_item)] list_item,
            move |_| {
                if let Some(video_segment) = list_item.item().and_downcast::<VideoSegment>() {
                    println!("Changed focus");
                    let selection_model = column_view.model().and_downcast::<SingleSelection>().unwrap();
                    let model = selection_model.model().and_downcast::<ListStore>().unwrap();
                    let row_index = model.find(&video_segment).unwrap_or(u32::MAX);
                    if row_index != u32::MAX {
                        selection_model.select_item(row_index, true);
                    }
                }
            }
        ));
        list_item.set_child(Some(&entry));
    });
    
    // Binds the stored data to the displayed entry objects
    factory.connect_bind(move |_factory, list_item: &ListItem| {
        let entry = list_item.child().unwrap().downcast::<Entry>().expect("The child is not an Entry");
        let item = list_item.item();
        let video_segment = item.and_downcast_ref::<VideoSegment>().expect("Item is not a VideoSegment");
        let current_name = video_segment.get_name();
        entry.set_text(&current_name);
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

fn build_start_time_offset_column(column_view: &gtk::ColumnView, start_time_offset_model: &ListStore, split_table_model: &ListStore, title: &str) {
    let factory = gtk::SignalListItemFactory::new();
    let start_time_offset_model_clone = start_time_offset_model.clone();
    let split_table_model_clone = split_table_model.clone();
    // Creates the entry objects
    factory.connect_setup(move |_, list_item| {
        let entry = gtk::Entry::new();
        entry.add_css_class("flat");
        entry.set_hexpand(true);
        entry.set_halign(gtk::Align::Fill);

        entry.connect_activate(glib::clone!(
            #[weak(rename_to = start_time_offset_model)] start_time_offset_model_clone,
            #[weak(rename_to = list_item)] list_item,
            #[weak(rename_to = entry)] entry,
            #[weak(rename_to = split_table_model)] split_table_model_clone,
            move |_| {
                if let Some(time_entry) = list_item.item().and_downcast::<TimeEntry>() {
                    let valid_entry = validate_split_table_entry(&entry);
                    if !valid_entry {
                        let time_entry_data = time_entry.get_time();
                        entry.set_text(format_clock(time_entry_data).as_str());
                    } else {
                        let new_time = string_to_nseconds(&entry.text().to_string()).unwrap();
                        time_entry.set_time(new_time);
                        let video_player_index = start_time_offset_model.find(&time_entry).unwrap();
                        for i in 0..split_table_model.n_items() {
                            let video_segment = split_table_model.item(i).and_downcast::<VideoSegment>().unwrap();
                            video_segment.set_offset(video_player_index as usize, new_time);
        
                        }
                        //update_times(&split_table_model, video_player_index, 0);
                    }
                }
            }
        ));
        list_item.set_child(Some(&entry));
    });

    // Binds the stored data to the displayed entry objects
    factory.connect_bind(move |_, list_item| {
        let item = list_item.item().and_then(|obj| obj.downcast::<TimeEntry>().ok()).expect("The item is not a VideoSegment");
        let entry = list_item.child().and_downcast::<Entry>().expect("The child widget is not entry");
        // Binds the value in the time entry to the entry text field
        // Any changes to the time entries value will be updated in the entry object
        let binding = item.bind_property("time", &entry, "text")
            .flags(glib::BindingFlags::DEFAULT | glib::BindingFlags::SYNC_CREATE)
            .transform_to(|_, value: u64| { // Converts the u64 time to formatted MM:SS.sss time to display
                Some(format_clock(value).to_value())
            })
            .build();

    });

    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column_view.append_column(&column);
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

        gio::resources_register_include!("spanel.gresource")
            .expect("Failed to register split planel resource.");

        gio::resources_register_include!("seekbar.gresource")
            .expect("Failed to register seek bar resource.");

        gio::resources_register_include!("sharedseekbar.gresource")
            .expect("Failed to register shared seek bar resource.");
        
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