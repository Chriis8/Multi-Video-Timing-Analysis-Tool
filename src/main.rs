mod video_pipeline;
use gio::ListStore;
use glib::{random_int_range, ExitCode, prelude::ObjectExt};
use gstreamer::ClockTime;
use gtk::{ColumnViewColumn, FlowBox, FlowBoxChild, ListItem, SelectionMode, SingleSelection};
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
    //let column_view: ColumnView = column_builder.object("column_view").expect("Failed to get column_view from UI File");
    let column_view_container: Box = builder.object("split_container").expect("Failed to column_view_container from UI File");
    //let add_column_button: Button = builder.object("add_column_button").expect("Failed to get add_column_button from UI File");
    let add_row_button: Button  = builder.object("add_row_button").expect("Failed to get add_row_button from UI File");
    let remove_column_button: Button = builder.object("remove_column_button").expect("Failed to get remove_column_button from UI File");
    let remove_row_button: Button = builder.object("remove_row_button").expect("Failed to get remove_row_button from UI File");
    let video_container: FlowBox = builder.object("video_container").expect("Failed to get video_container from UI File");
    
    let (model, column_view) = create_column_view();
    column_view_container.append(&column_view);

    let column_view_clone = column_view.clone();
    add_name_column(&column_view_clone, "Segment Name");

    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    add_row_button.connect_clicked(move |_| { //adds an item to liststore
        add_empty_row(&column_view_clone, &model_clone);
    });

    let column_view_clone = column_view.clone();
    remove_column_button.connect_clicked(move |_| {
        remove_column(&column_view_clone);
    });

    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    remove_row_button.connect_clicked(move |_| {
        if let Some(selection_model) = column_view_clone.model().and_downcast::<SingleSelection>() {
            let selected_index = selection_model.selected();
            println!("Removing Row {selected_index}");
            remove_row(&model_clone, selected_index);
        } else {
            eprintln!("Couldnt get selection model");
        }
    });

    
    let initial_child_count = 0_usize;
    store_data(&video_container, "count", initial_child_count);

    let button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let column_view_clone = column_view.clone();
    let model_clone = model.clone();
    let video_container_clone = video_container.clone();
    button.connect_clicked(move |_| {
        //let video_container: FlowBox = builder_clone.object("video_container").expect("failed to get video_container from UI file");
        let count = unsafe{ get_data::<usize>(&video_container_clone, "count").unwrap().as_ref() };
        println!("Count before operation: {count}");
        //let column_count = column_view_clone.columns().n_items() - 1;
        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        let new_player = VideoPlayer::new(*count as u32);
        new_player.setup_event_handlers(window);
        
        let model_clone_clone = model_clone.clone();
        let column_view_clone_clone = column_view_clone.clone();
        new_player.connect_local("button-clicked", false, move |args| {
            let id: u32 = args[1].get().unwrap();
            let position: u64 = args[2].get().unwrap();
            println!("Main Scope: button clicked ----- {id} | {position}");
            let row_count = model_clone_clone.n_items();
            println!("row count: {row_count}");
            
            let mut update_row = row_count;
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
            if update_row == row_count {
                add_row(&column_view_clone_clone, &model_clone_clone, position, position);
            } else {
                let segment = model_clone_clone.item(update_row).and_downcast::<VideoSegment>().unwrap();
                segment.set_segment(id as usize, position, position);
                model_clone_clone.remove(update_row);
                model_clone_clone.insert(update_row, &segment);
            }
            print_vec(&model_clone_clone);
            None
        });
        
        let name = random_int_range(0, 99);
        let model_clone_clone = model_clone.clone();
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), *count, SegmentField::Time);
        add_column(&column_view_clone, &model_clone_clone, name.to_string().as_str(), *count, SegmentField::Duration);
        for i in 0..model_clone_clone.n_items() {
            let seg = model_clone_clone.item(i).and_downcast::<VideoSegment>().unwrap();
            seg.add_segment(1, 2);
        }

        video_container_clone.set_homogeneous(true);
        let number_of_columns = (*count as u32 + 1).clamp(1,3);
        video_container_clone.set_max_children_per_line(number_of_columns);
        video_container_clone.set_min_children_per_line(number_of_columns);
        video_container_clone.set_valign(gtk::Align::Fill);
        video_container_clone.set_selection_mode(SelectionMode::None);
        video_container_clone.set_column_spacing(0);

        // let flow_child = gtk::FlowBoxChild::new();
        // flow_child.set_hexpand(true);
        // flow_child.set_halign(gtk::Align::Fill);
        // flow_child.set_css_classes(&["flow"]);
        // flow_child.set_child(Some(&new_player));
        video_container_clone.append(&new_player);

        store_data(&video_container_clone, "count", count + 1);
    });

    let button: Button = builder.object("print_splits_button").expect("Failed to get new split button");
    let model_clone = model.clone();
    button.connect_clicked(move |_| {
        print_vec(&model_clone);
    });

    app.add_window(&window);
    window.show();
    builder
}

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

fn clocktime_from_parts(minutes: String, seconds: String, subseconds: String) -> Option<ClockTime> {
    let minutes: u64 = minutes.parse().unwrap();
    let seconds: u64 = seconds.parse().unwrap();

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
    Some(ClockTime::from_nseconds(total_nanos))
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

fn add_column(column_view: &gtk::ColumnView, _model: &ListStore, title: &str, index: usize, field: SegmentField) {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, list_item| {
        let entry = gtk::Entry::new();
        list_item.set_child(Some(&entry));
    });
    
    factory.connect_bind(move |_, list_item| {
        let entry = list_item.child().unwrap().downcast::<gtk::Entry>().unwrap();
        if let Some(item) = list_item.item().and_downcast::<VideoSegment>() {
            match item.get_segment(index) {
                Some(data) => {
                    let text = match field {
                        SegmentField::Time => {
                            data.time.map_or("none".to_string(), |t| format_clock(ClockTime::from_nseconds(t)))
                        },
                        SegmentField::Duration => {
                            data.duration.map_or("none".to_string(), |d| format_clock(ClockTime::from_nseconds(d)))
                        }
                    };
                    entry.set_text(text.as_str());

                    entry.connect_changed(glib::clone!(
                        #[weak(rename_to = seg)] item,
                        #[strong] field,
                        move |entry| {
                            let mut text = entry.text().to_string();
                            let minutes: String = text.drain(..text.find(":").unwrap()).collect();
                            text.remove(0);
                            let seconds: String = text.drain(..text.find(".").unwrap()).collect();
                            text.remove(0);
                            let subsecond: String = text;
                            let time = clocktime_from_parts(minutes, seconds, subsecond).unwrap().nseconds();
                            match field {
                                SegmentField::Time => {
                                    seg.set_time(index, time);
                                },
                                SegmentField::Duration => {
                                    seg.set_duration(index, time);
                                }
                            }
                            
                        }
                    ));
                } 
                None => entry.set_text("Empty"),
            };
        }
    });
    
    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column_view.append_column(&column);
}

fn add_name_column(column_view: &gtk::ColumnView, title: &str) {
    let factory = gtk::SignalListItemFactory::new();
    
    factory.connect_setup(|_factory, list_item: &ListItem| {
        let entry = Entry::new();
        
        list_item.set_child(Some(&entry));
    });
    
    factory.connect_bind(|_factory, list_item: &ListItem| {
        let entry = list_item.child().unwrap().downcast::<Entry>().expect("The child is not an Entry");
        let item = list_item.item();
        let video_segment = item.and_downcast_ref::<VideoSegment>().expect("Item is not a VideoSegment");
        let current_name = video_segment.get_name();
        entry.set_text(&current_name);
        
        entry.connect_changed(glib::clone!(
            #[weak(rename_to = seg)] video_segment,
            move |entry| {
                let new_name = entry.text().to_string();
                seg.set_name(new_name);
            } 
        ));
    });
    
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

fn add_row(column_view: &ColumnView, model: &ListStore, time: u64, duration: u64) {
    let column_count = column_view.columns().n_items() - 1;
    let row_count = model.n_items() as usize;
    let name = row_count;
    let seg = VideoSegment::new(name.to_string().as_str());
    
    for _ in 0..column_count {
        seg.add_segment(time, duration);
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

fn store_data<T: 'static>(widget: &impl ObjectExt, key: &str, value: T) {
    unsafe {
        widget.set_data(key, value);
    }
}

fn get_data<T: 'static>(widget: &impl ObjectExt, key: &str) -> Option<std::ptr::NonNull<T>> {
    unsafe { widget.data::<T>(key) }
}

fn main() -> glib::ExitCode {
    let run_app = true;
    if run_app {
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
    } else {
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
