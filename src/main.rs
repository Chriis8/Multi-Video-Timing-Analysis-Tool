mod video_pipeline;
use gio::ListStore;
use glib::random_int_range;
use gtk::{ColumnViewColumn, ListItem};
use gtk::{ gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button, ColumnView, CssProvider, Label, StringObject, Entry};
use gstgtk4;
mod widgets;
use widgets::video_player_widget::video_player::VideoPlayer;
use widgets::split_panel::splits::VideoSegment;

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
    let column_builder = Builder::from_resource("/spanel/spanel.ui");

    load_css("src\\widgets\\main_window\\style.css");
    load_css("src\\widgets\\split_panel\\style.css");

    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
    let test_buttons: Vec<Button> = Vec::new();
    //let column_view: ColumnView = column_builder.object("column_view").expect("Failed to get column_view from UI File");
    let column_view_container: Box = builder.object("split_container").expect("Failed to column_view_container from UI File");
    let add_column_button: Button = builder.object("add_column_button").expect("Failed to get add_column_button from UI File");
    let add_row_button: Button  = builder.object("add_row_button").expect("Failed to get add_row_button from UI File");
    let remove_column_button: Button = builder.object("remove_column_button").expect("Failed to get remove_column_button from UI File");
    let remove_row_button: Button = builder.object("remove_row_button").expect("Failed to get remove_row_button from UI File");

    let (model, column_view) = create_column_view();

    column_view_container.append(&column_view);

    let column_view_clone = column_view.clone();
    add_name_column(&column_view_clone, "Segment Name");
    
    let column_view_clone = column_view.clone();
    let model_clone = model.clone();
    add_column_button.connect_clicked(move |_| { //add new item to each item in the liststore
        let name = random_int_range(0, 99);
        let column_count = column_view_clone.columns().n_items() - 1;

        println!("NUmber of columns not including name: {column_count} (Before)");

        for i in 0..model_clone.n_items() {
            let seg = model_clone.item(i).and_downcast::<VideoSegment>().unwrap();
            let time = random_int_range(i as i32 * 100, (i as i32 + 1) * 100);
            let duration = random_int_range(i as i32 * 100, (i as i32 + 1) * 100); 
            seg.add_segment(time as u64, duration as u64);
        }

        add_column(&column_view_clone, name.to_string().as_str(), column_count as usize);
    });

    let model_clone = model.clone();
    let column_view_clone = column_view.clone();
    add_row_button.connect_clicked(move |_| { //adds an item to liststore
        let column_count = column_view_clone.columns().n_items() - 1;
        let row_count = model_clone.n_items() as usize;
        let name = row_count;
        let seg = VideoSegment::new(name.to_string().as_str());
        
        for _ in 0..column_count {
            let time = random_int_range(row_count as i32 * 100, (row_count as i32 + 1) * 100);
            let duration = random_int_range(row_count as i32 * 100, (row_count as i32 + 1) * 100);
            seg.add_segment(time as u64, duration as u64);
        }

        add_row(&model_clone, seg);
    });

    let column_view_clone = column_view.clone();
    remove_column_button.connect_clicked(move |_| {
        remove_column(&column_view_clone);
    });

    let model_clone = model.clone();
    remove_row_button.connect_clicked(move |_| {
        remove_row(&model_clone);
    });

    let button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    let builder_clone = builder.clone();
    let column_view_clone = column_view.clone();
    let model_clone = model.clone();
    button.connect_clicked(move |_| {
        let video_container: Box = builder_clone.object("video_container").expect("failed to get video_container from UI file");
        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        let new_player = VideoPlayer::new();
        new_player.setup_event_handlers(window);
        video_container.append(&new_player);

        let name = random_int_range(0, 99);
        let column_count = column_view_clone.columns().n_items() - 1;
        add_column(&column_view_clone, name.to_string().as_str(), column_count as usize);
        
        let new_test_button = new_player.test_button();
        let model_clone = model_clone.clone();
        new_test_button.connect_clicked(move |_| {
            println!("clicked test button {column_count}");
            let gstman_weak = new_player.pipeline();
            if let Some(gstman) = gstman_weak.upgrade() {
                if let Ok(mut guard) = gstman.lock() {
                    if let Some(ref mut pipeline) = *guard {
                        let row_count = model_clone.n_items() - 1;
                        println!("{row_count}");
                        let segment = model_clone.item(row_count).and_downcast::<VideoSegment>().unwrap();
                        let current_time = pipeline.get_position().unwrap();
                        let nanos = current_time.nseconds();
                        segment.set_segment(column_count as usize, nanos, nanos);
                        model_clone.remove(row_count);
                        model_clone.insert(row_count, &segment);
                        //model_clone.items_changed(row_count, 1, 1);
                    } else {
                        eprintln!("No Video Pipeline available");
                    }
                } else {
                    eprintln!("Failed to aquire lock on Video pipeline");
                }
            } else {
                eprintln!("OAOSOFASD");
            }
        });
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
                let time = item.get_time(j).to_string();
                let duration = item.get_duration(j).to_string();
                print!("{time}, {duration} |");
            }
        }
        println!("");
    }
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

fn add_column(column_view: &gtk::ColumnView, title: &str, index: usize) {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, list_item| {
        let label = gtk::Label::new(None);
        list_item.set_child(Some(&label));
    });
    
    factory.connect_bind(move |_, list_item| {
        let label = list_item.child().unwrap().downcast::<gtk::Label>().unwrap();
        if let Some(item) = list_item.item().and_downcast::<VideoSegment>() {
            let data1 = &item.get_time(index);
            let data2 = &item.get_duration(index);
            let text = format!("{data1} {data2}");
            label.set_text(text.as_str());
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

fn add_row(model: &gio::ListStore, segment: VideoSegment) {
    model.append(&segment);
}

fn remove_row(model: &gio::ListStore) {
    if model.n_items() > 0 {
        model.remove(model.n_items() - 1);
    }
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

    gio::resources_register_include!("spanel.gresource")
        .expect("Failed to register resources.");
    
    let app = gtk::Application::new(None::<&str>, gtk::gio::ApplicationFlags::FLAGS_NONE);
    app.connect_activate(|app| {
        
        let builder = build_ui(app);

        // let window = ApplicationWindow::new(app);

        // window.set_default_size(800, 600);
        // window.set_title(Some("Video Player"));

        // let main_box = Box::new(gtk::Orientation::Horizontal, 10);

        

        // window.set_child(Some(&main_box));

        // window.show();
        let builder_clone = builder.clone();
        app.connect_shutdown(move |_| {
            println!("shutting down");
            let video_container: Box = builder_clone.object("video_container").expect("failed to get video_container from UI file");
            
            while let Some(child) = video_container.last_child() {
                let video = child.downcast::<VideoPlayer>().unwrap();
                unsafe {
                    video.unparent(); 
                    video.run_dispose();
                }
            }
        });

        // app.add_window(&window);
        // window.show();

    });


    let res = app.run();

    unsafe {
        gstreamer::deinit();
    }
    res
}
