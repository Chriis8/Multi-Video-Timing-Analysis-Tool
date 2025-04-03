mod video_pipeline;
use gio::ListStore;
use glib::random_int_range;
use gtk::{ColumnViewColumn, ListItem};
use gtk::{ gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button, ColumnView, CssProvider, Label, StringObject, Entry};
use gstgtk4;
use std::{rc::Rc, cell::RefCell};
mod widgets;
use widgets::video_player_widget::video_player::VideoPlayer;
use widgets::split_panel::splits::VideoSegment;

struct Videos {
    video_players: std::cell::RefCell<Vec<Vec<VideoSegment>>>,
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

fn build_ui(app: &Application, videos: Rc<Videos>) -> Builder {
    let builder = Builder::from_resource("/mainwindow/mwindow.ui");
    let column_builder = Builder::from_resource("/spanel/spanel.ui");

    load_css("src\\widgets\\main_window\\style.css");
    load_css("src\\widgets\\split_panel\\style.css");

    let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
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
    add_column_button.connect_clicked(move |_| {
        let name = random_int_range(0, 99);
        add_column(&column_view_clone, name.to_string().as_str(), |seg| seg.get_time().to_string());
        add_column(&column_view_clone, name.to_string().as_str(), |seg| seg.get_duration().to_string());
    });

    let model_clone = model.clone();
    let videos_clone = videos.clone();
    add_row_button.connect_clicked(move |_| {
        let row_count = model_clone.n_items() as usize;
        // let name = videos_clone.video_players.borrow()[row_count].get_name();
        // let time = videos_clone.video_players.borrow()[row_count].get_time();
        // let duration = videos_clone.video_players.borrow()[row_count].get_duration();
        // add_row(&model_clone, name.as_str(), time, duration);
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
    let videos_clone = videos.clone();
    button.connect_clicked(move |_| {
        let video_container: Box = builder_clone.object("video_container").expect("failed to get video_container from UI file");
        let window: ApplicationWindow = builder_clone.object("main_window").expect("Failed to get main_window from UI file");
        let new_player = VideoPlayer::new();
        new_player.setup_event_handlers(window);
        video_container.append(&new_player);

        new_row(&videos_clone);

        let name = random_int_range(0, 99);
        add_column(&column_view_clone, name.to_string().as_str(), |seg| seg.get_time().to_string());
        add_column(&column_view_clone, name.to_string().as_str(), |seg| seg.get_duration().to_string());
    });

    let button: Button = builder.object("new_split").expect("Failed to get new split button");
    let videos_clone = videos.clone();
    button.connect_clicked(move |_| {
        for row in &mut *videos_clone.video_players.borrow_mut() {
            let name = random_int_range(100, 999).to_string();
            let time = random_int_range(1000, 9999) as u64;
            let duration = random_int_range(10000, 99999) as u64;
            row.push(VideoSegment::new(name.as_str(), time, duration));
        }
    });

    let button: Button = builder.object("print_splits_button").expect("Failed to get new split button");
    let videos_clone = videos.clone();
    button.connect_clicked(move |_| {
        print_vec(&videos_clone);
    });

    app.add_window(&window);
    window.show();
    builder
}

fn print_vec(v: &Videos) {
    println!("Splits");
    for row in &*v.video_players.borrow() {
        for segment in row {
            let name = segment.get_name();
            print!("{name} | ");
        }
        println!("");
    }
}

fn new_row(v: &Videos) {
    if v.video_players.borrow().len() == 0 {
        let new_row: Vec<VideoSegment> = Vec::new();
        v.video_players.borrow_mut().push(new_row);
        return;
    }

    let length = v.video_players.borrow()[0].len();
    let mut new_row: Vec<VideoSegment> = Vec::new();
    
    for _ in 0..length {
        let name = random_int_range(100, 999).to_string();
        let time = random_int_range(1000, 9999) as u64;
        let duration = random_int_range(10000, 99999) as u64;
        new_row.push(VideoSegment::new(name.as_str(), time, duration));
    }

    v.video_players.borrow_mut().push(new_row);
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
        
        let videos: Rc<Videos> = Rc::new(Videos {
            video_players: RefCell::new(Vec::new()),
        });
        let builder = build_ui(app, videos);

        // let window = ApplicationWindow::new(app);

        // window.set_default_size(800, 600);
        // window.set_title(Some("Video Player"));

        // let main_box = Box::new(gtk::Orientation::Horizontal, 10);

        

        // window.set_child(Some(&main_box));

        // window.show();


        // app.add_window(&window);
        // window.show();

    });

    let res = app.run();

    unsafe {
        gstreamer::deinit();
    }
    res
}

fn create_factory() -> gtk::SignalListItemFactory{
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(|_, item: &gtk::ListItem| {
        let label = Label::new(None);
        item.set_child(Some(&label));
    });

    factory.connect_bind(|_, item: &gtk::ListItem| {
        let label = item.child().unwrap().downcast::<Label>().unwrap();

        let string_obj = item.item().unwrap().downcast::<StringObject>().unwrap().string();
        label.set_text(&string_obj);
    });

    factory
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

fn add_column(column_view: &gtk::ColumnView, title: &str, field_accessor: fn(&VideoSegment) -> String) {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, list_item| {
        let label = gtk::Label::new(None);
        list_item.set_child(Some(&label));
    });

    factory.connect_bind(move |_, list_item| {
        let label = list_item.child().unwrap().downcast::<gtk::Label>().unwrap();
        let item = list_item.item().and_downcast::<VideoSegment>();

        if let Some(item) = item {
            label.set_text(&field_accessor(&item));
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

fn add_row(model: &gio::ListStore, name: &str, time: u64, duration: u64) {
    let segment = VideoSegment::new(name, time, duration);
    model.append(&segment);
}

fn remove_row(model: &gio::ListStore) {
    if model.n_items() > 0 {
        model.remove(model.n_items() - 1);
    }
}
