use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use std::sync::{Arc, Mutex};
use crate::video_pipeline::VideoPipeline;

mod imp {
    use gtk::{Button, Box, Label, Picture, template_callbacks};
    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/videoplayer/video_player.ui")]
    pub struct VideoPlayer {
        pub gstreamer_manager: Arc<Mutex<Option<VideoPipeline>>>,        
        
        #[template_child]
        pub vbox: TemplateChild<Box>,

        #[template_child]
        pub hboxtop: TemplateChild<Box>,

        #[template_child]
        pub fchooser: TemplateChild<Button>,

        #[template_child]
        pub text_view: TemplateChild<Label>,

        #[template_child]
        pub picture: TemplateChild<Picture>,

        #[template_child]
        pub label: TemplateChild<Label>,

        #[template_child]
        pub hbox: TemplateChild<Box>,

        #[template_child]
        pub previous_frame_button: TemplateChild<Button>,
        
        #[template_child]
        pub play_button: TemplateChild<Button>,
        
        #[template_child]
        pub stop_button: TemplateChild<Button>,
        
        #[template_child]
        pub next_frame_button: TemplateChild<Button>,
        
        #[template_child]
        pub test_button: TemplateChild<Button>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoPlayer {
        const NAME: &'static str = "VideoPlayer";
        type Type = super::VideoPlayer;
        type ParentType = gtk::Box;
        
        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    
    impl ObjectImpl for VideoPlayer {
        fn constructed(&self) {
            self.parent_constructed();

            // self.fchooser.connect_clicked(glib::clone!(
            //     #[weak(rename_to = gstman)] self.gstreamer_manager,
            //     #[weak(rename_to = text)] self.text_view,
            //     #[weak(rename_to = pic)] self.picture,
            //     #[weak(rename_to = win)] self.obj().ancestor(gtk::Window::static_type()).unwrap(),
            //     move |fchooser| {
            //         let videos_filter = gtk::FileFilter::new();
            //         videos_filter.set_name(Some("Video Files"));
            //         videos_filter.add_pattern("*.mp4");   // MP4 format
            //         // Add additional video formats here

            //         let dialog = gtk::FileChooserDialog::builder()
            //             .title("Open File")
            //             .action(gtk::FileChooserAction::Open)
            //             .modal(true)
            //             .filter(&videos_filter)
            //             .build();
            //         dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            //         dialog.add_button("Accept", gtk::ResponseType::Accept);
            //         if let Some(window) = win.downcast::<gtk::Window>().ok() {
            //             dialog.set_transient_for(Some(&window));
            //         }
            //     }
            // ));
        }


    }
    impl WidgetImpl for VideoPlayer {}
    impl BoxImpl for VideoPlayer {}
    
}

glib::wrapper! {
    pub struct VideoPlayer(ObjectSubclass<imp::VideoPlayer>)
    @extends gtk::Widget,
    @implements gtk::Buildable;
}

impl VideoPlayer {
    pub fn new() -> Self {
        let widget: Self = glib::Object::new::<Self>();
        
        let imp = imp::VideoPlayer::from_obj(&widget);
        
        if let Ok(mut pipeline) = imp.gstreamer_manager.lock() {
            *pipeline = Some(VideoPipeline::new());
        }

        eprint!("created video player widget");
        widget
    }

    pub fn setup_event_handlers(&self) {
        let imp = imp::VideoPlayer::from_obj(self);

        eprintln!("Setting up buttons");

        imp.fchooser.connect_clicked(glib::clone!(
            #[weak(rename_to = gstman)] imp.gstreamer_manager,
            #[weak(rename_to = text)] imp.text_view,
            #[weak(rename_to = pic)] imp.picture,
            #[weak(rename_to = win)] imp.obj().ancestor(gtk::ApplicationWindow::static_type()).unwrap(),
            move |fchooser| {
                let videos_filter = gtk::FileFilter::new();
                videos_filter.set_name(Some("Video Files"));
                videos_filter.add_pattern("*.mp4");   // MP4 format
                // Add additional video formats here

                let dialog = gtk::FileChooserDialog::builder()
                    .title("Open File")
                    .action(gtk::FileChooserAction::Open)
                    .modal(true)
                    .filter(&videos_filter)
                    .build();
                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Accept", gtk::ResponseType::Accept);
                if let Some(window) = win.downcast::<gtk::ApplicationWindow>().ok() {
                    dialog.set_transient_for(Some(&window));
                } else {
                    eprintln!("OOOOOOOF");
                }

                dialog.run_async(move |obj, res| {
                    match res {
                        gtk::ResponseType::Accept => {
                            println!("Accepted");
                            if let Some(file) = obj.file() {
                                let from_str = gtk::gio::File::uri(&file);
                                println!("from_str {from_str}");
                                text.set_label(&from_str);
                                println!("File accepted: {}", from_str);
                                if let Ok(mut guard) = gstman.lock() {
                                    if let Some(ref mut pipeline) = *guard {
                                        pipeline.reset();
                                        pipeline.build_pipeline(Some(&text.label().to_string()));
                                        let paintable = pipeline.get_paintable();
                                        pic.set_paintable(Some(&paintable));
                                    } else {
                                        eprintln!("No Video Pipeline available");
                                    }
                                } else {
                                    eprintln!("Failed to aquire lock on Video pipeline");
                                }
                            }
                        }
                        _ => {
                            println!("No file selected");
                        }
                    }
                    obj.destroy();
                });
            }
        ));
        
    }
}
