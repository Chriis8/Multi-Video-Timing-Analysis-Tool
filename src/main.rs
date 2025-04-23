mod video_pipeline;
use gio::ListStore;
use glib::{random_int_range, ExitCode, prelude::ObjectExt, Regex, RegexCompileFlags, RegexMatchFlags};
use gstreamer::ClockTime;
use gtk::{ColumnViewColumn, EventControllerFocus, FlowBox, FlowBoxChild, ListItem, SelectionMode};
use gtk::{ gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button, ColumnView, CssProvider, Entry, Label};
use gstgtk4;
mod widgets;
use widgets::video_player_widget::video_player::VideoPlayer;
use widgets::split_panel::splits::VideoSegment;

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

fn build_ui(app: &Application) -> Builder {
    let builder = Builder::from_resource("/mainwindow/mwindow.ui");
    let _column_builder = Builder::from_resource("/spanel/spanel.ui");

    load_css("src\\widgets\\main_window\\style.css");
    load_css("src\\widgets\\split_panel\\style.css");

    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
    let column_view_container: Box = builder.object("split_container").expect("Failed to column_view_container from UI File");
    let video_container: FlowBox = builder.object("video_container").expect("Failed to get video_container from UI File");

    video_container.set_homogeneous(true);
    video_container.set_valign(gtk::Align::Fill);
    video_container.set_selection_mode(SelectionMode::None);
    video_container.set_column_spacing(0);

    let (model, column_view) = create_column_view();
    column_view_container.append(&column_view);

    // Adds first row of segment names to the split table
    let column_view_clone = column_view.clone();
    add_name_column(&column_view_clone, "Segment Name");
    
    // Add data to video_container to keep track of the number of active videos
    let initial_child_count = 0_usize;
    store_data(&video_container, "count", initial_child_count);

    let button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let column_view_clone = column_view.clone();
    let model_clone = model.clone();
    let video_container_clone = video_container.clone();
    // Adds new video player and new columns to split table
    button.connect_clicked(move |_| {
        let count = unsafe{ get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() };
        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        
        // Sets up new video player
        let new_player = VideoPlayer::new(*count as u32);
        new_player.setup_event_handlers(window);
        
        let model_clone_clone = model_clone.clone();
        let column_view_clone_clone = column_view_clone.clone();
        // Listens to the split button from a video player
        // args[1] ID u32: index from the video player thats button was pressed
        // args[2] Position u64: time in nano seconds that the video player playback head was at when the button was pressed
        new_player.connect_local("button-clicked", false, move |args| {
            let id: u32 = args[1].get().unwrap();
            let position: u64 = args[2].get().unwrap();
            let row_count = model_clone_clone.n_items();
            
            let mut update_row = row_count;
            // Finds first row without a time and duration
            for i in 0..row_count {
                let segment = model_clone_clone.item(i).and_downcast::<VideoSegment>().unwrap();
                match segment.get_segment(id as usize) {
                    Some(data) => {
                        let time = data.time;
                        let duration = data.duration;
                        if let (Some(_t), Some(_d)) = (time, duration) {
                            continue
                        } else {
                            update_row = i;
                            break
                        }
                    }
                    None => break
                };
            }
            // New row is added if there are no empty rows
            if update_row == row_count {
                add_empty_row(&column_view_clone_clone, &model_clone_clone);
            }
            // Updates cell information at row: update_row and column: id with the new position and duration
            let segment = model_clone_clone.item(update_row).and_downcast::<VideoSegment>().unwrap();
            segment.set_segment(id as usize, position, position);
            // Updates the table dislay
            model_clone_clone.remove(update_row);
            model_clone_clone.insert(update_row, &segment);
            None
        });
        
        // Adds two columns to split table for each new video player
        // Column 1: (Time) Split time -> time since the start of the clip
        // Column 2: (Duration) Segment time -> time since the last split
        let name = random_int_range(0, 99);
        let model_clone_clone = model_clone.clone();
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), *count, SegmentField::Time);
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), *count, SegmentField::Duration);
        // Updates the data in the liststore to include the two new rows with empty data
        for i in 0..model_clone_clone.n_items() {
            let seg = model_clone_clone.item(i).and_downcast::<VideoSegment>().unwrap();
            seg.add_segment(1, 2);
        }

        // Updates formatting of the video players and adds the new video player to the container
        let number_of_columns = (*count as u32 + 1).clamp(1,3);
        video_container_clone.set_max_children_per_line(number_of_columns);
        video_container_clone.set_min_children_per_line(number_of_columns);
        video_container_clone.append(&new_player);

        // Updates video_container data keeping track of the active video players
        store_data(&video_container_clone, "count", count + 1);
    });

    // Debug function to print the split data in liststore
    // Used to make sure the split data is correctly being stored as this is separate from the displayed information in the table
    let button: Button = builder.object("print_splits_button").expect("Failed to get new split button");
    let model_clone = model.clone();
    button.connect_clicked(move |_| {
        print_vec(&model_clone);
    });

    app.add_window(&window);
    window.show();
    builder
}

// Used to make sure the split data is correctly being stored as this is separate from the displayed information in the table
fn print_vec(model: &ListStore) {
    println!("Splits");
    for i in 0..model.n_items() {
        print!("Row: {i} ");
        if let Some(item) = model.item(i).and_downcast::<VideoSegment>() {
            for j in 0..item.count() {
                match item.get_segment(j) {
                    Some(data) => {
                        let time = data.time.map_or(String::from("None"), |v| v.to_string());
                        let duration = data.duration.map_or(String::from("None"), |v| v.to_string());
                        print!("{time}, {duration} |");
                    } 
                    None => print!("x |")
                };
            }
        }
        println!("");
    }
}

// Converts a GStreamer ClockTime to a String
// Format: MM:SS.sss or HH:MM:SS.sss if hours exist
fn format_clock(time: ClockTime) -> String {
    let mut ret = time.to_string();
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
    selection_model.set_autoselect(false);
    selection_model.set_can_unselect(true);
    
    // Create the ColumnView
    let column_view = gtk::ColumnView::new(Some(selection_model));
    
    (model, column_view)
}

// Add new column to the column view
// index: video player id NOT the column index 
// field: Time or Duration
// Each video player gets two columns one for time and one for duration
fn add_column(column_view: &gtk::ColumnView, _model: &ListStore, title: &str, index: usize, field: SegmentField) {
    let factory = gtk::SignalListItemFactory::new();
    // Creates the entry objects
    factory.connect_setup(move |_, list_item| {
        let entry = gtk::Entry::new();
        list_item.set_child(Some(&entry));
    });

    // Binds the stored data to the displayed entry objects
    let model_clone = _model.clone();
    factory.connect_bind(move |_, list_item| {
        // Get Entry object
        let entry = list_item.child().unwrap().downcast::<gtk::Entry>().unwrap();
        if let Some(item) = list_item.item().and_downcast::<VideoSegment>() {
            let row_index = model_clone.find(&item).unwrap();
            match item.get_segment(index) {
                Some(data) => {
                    // Gets the formatted clocktime of the specified field for the column
                    let text = match field {
                        SegmentField::Time => {
                            data.time.map_or("none".to_string(), |t| format_clock(ClockTime::from_nseconds(t)))
                        },
                        SegmentField::Duration => {
                            data.duration.map_or("none".to_string(), |d| format_clock(ClockTime::from_nseconds(d)))
                        }
                    };
                    // Updates the entry object text to display the time
                    entry.set_text(text.as_str());
                    let t = text.as_str();
                    println!("Set {t} to column {index}, row {row_index}");
                    // Stores the time as the last valid time to be used to rollback to if invalid time is manually entered by user
                    store_data(&entry, "data", text);

                    let focus_control = EventControllerFocus::new();
                    
                    // Signal for user to submit (Pressing Enter) manual edits to a cell in split table
                    entry.connect_activate(glib::clone!(
                        #[weak(rename_to = seg)] item,
                        #[strong] field,
                        #[weak(rename_to = entry)] entry,
                        move |_| {
                            // Validates the formatting of the user inputted time and updates the segment data and entry information
                            validate_and_apply(&entry, &field, &seg, index);
                        }
                    ));
                    
                    // Signal for when user manually edits a cell in split table and leaves focus
                    // Same as if the user submitted the edit
                    focus_control.connect_leave(glib::clone!(
                        #[weak(rename_to = seg)] item,
                        #[strong] field,
                        #[weak(rename_to = entry)] entry,
                        move |_| {
                            // Validates the formatting of the user inputted time and updates the segment data and entry information
                            validate_and_apply(&entry, &field, &seg, index);
                        }
                    ));
                    
                    // Adds the focus controller to the entry object
                    entry.add_controller(focus_control);
                } 
                None => entry.set_text("Empty"),
            };
        }
    });

    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column_view.append_column(&column);
}

// Validates the formatting of the user inputted time and updates the segment data and entry information
// Index: video player id NOT the column index 
// Field: Time or Duration
// Each video player gets two columns one for time and one for duration
fn validate_and_apply(entry: &Entry, field: &SegmentField, seg: &VideoSegment, index: usize) {
    let input = entry.text().to_string();
    let pattern = r"^[0-5][0-9]:[0-5][0-9]\.\d{3}$";
    // Checks if the input matches the format: MM:SS.sss
    let re = Regex::match_simple(pattern, input.clone(), RegexCompileFlags::empty(), RegexMatchFlags::empty());
    if !re {
        println!("Entry is not in valid format");
        // Gets previously saved valid data to reset the users changes
        let previous_text = unsafe { get_data::<String>(entry, "data").unwrap().as_ref() };
        entry.set_text(previous_text.as_str());
        return;
    }

    // Converts user input to nano seconds and updates segment data
    let time = string_to_nseconds(&input).unwrap();
    match field {
        SegmentField::Time => {
            seg.set_time(index, time);
        },
        SegmentField::Duration => {
            seg.set_duration(index, time);
        }
    }
    println!("Stored value {input} to entry");
    // Updates entry data to store the new rollback time if invalid time is manually entered by user
    store_data(entry, "data", input);
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

// Adds empty row to column view
fn add_empty_row(column_view: &ColumnView, model: &ListStore) {
    let column_count = column_view.columns().n_items() - 1;
    let row_count = model.n_items() as usize;
    let name = row_count;
    let seg = VideoSegment::new(name.to_string().as_str());
    
    for _ in 0..column_count {
        seg.add_empty_segment();
    }
    model.append(&seg);
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
    } else if run_app == 2 {
    }
    
    ExitCode::SUCCESS
}
