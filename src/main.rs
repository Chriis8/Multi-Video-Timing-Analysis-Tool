mod video_pipeline;
use gio::ListStore;
use glib::{random_int_range, uuid_string_random};
use gtk::ColumnViewColumn;
use gtk::{ gdk::Display, glib, prelude::*, Application, ApplicationWindow, Box, Builder, Button, ColumnView, CssProvider, Label, StringObject};
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
    //let column_view: ColumnView = column_builder.object("column_view").expect("Failed to get column_view from UI File");
    let column_view_container: Box = builder.object("split_container").expect("Failed to column_view_container from UI File");
    let add_column_button: Button = builder.object("add_column_button").expect("Failed to get add_column_button from UI File");
    let add_row_button: Button  = builder.object("add_row_button").expect("Failed to get add_row_button from UI File");
    let remove_column_button: Button = builder.object("remove_column_button").expect("Failed to get remove_column_button from UI File");
    let remove_row_button: Button = builder.object("remove_row_button").expect("Failed to get remove_row_button from UI File");

    let (model, column_view) = create_column_view();

    column_view_container.append(&column_view);

    let column_view_clone = column_view.clone();
    add_column_button.connect_clicked(move |_| {
        let name = random_int_range(0, 100);
        add_column(&column_view_clone, name.to_string().as_str(), |seg| seg.get_name());
    });

    let model_clone = model.clone();
    add_row_button.connect_clicked(move |_| {
        add_row(&model_clone, "Segment", 1000, 5000);
    });

    let column_view_clone = column_view.clone();
    remove_column_button.connect_clicked(move |_| {
        remove_column(&column_view_clone);
    });

    let model_clone = model.clone();
    remove_row_button.connect_clicked(move |_| {
        remove_row(&model_clone);
    });

    app.add_window(&window);
    window.show();
    builder
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
        set_up_button(builder);

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

fn set_up_button(builder: Builder) {
    let button: Button = builder.object("new_video_player_button").expect("Failed to get button");
    
    button.connect_clicked(move |_| {
        let video_container: Box = builder.object("video_container").expect("failed to get video_container from UI file");
        let window: ApplicationWindow = builder.object("main_window").expect("Failed to get main_window from UI file");
        let new_player = VideoPlayer::new();
        new_player.setup_event_handlers(window);
        video_container.append(&new_player);
    });
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
