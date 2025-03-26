use glib::timeout_add_local;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::CssProvider;
use gtk::gdk::Display;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::video_pipeline::VideoPipeline;

mod imp {

    use gtk::{Box, Button, Label, Picture, Scale};
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
        pub scale_parent: TemplateChild<Box>,

        #[template_child]
        pub seek_bar: TemplateChild<Scale>,

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

    impl VideoPlayer {
        fn setup_seek_bar(&self) {
            println!("Setting up seek bar!");

            let adjustment = gtk::Adjustment::new(0.0, 0.0, 100.0, 1.0, 0.0, 0.0);
            self.seek_bar.set_adjustment(&adjustment);
            //self.seek_bar.add_mark(10.0, gtk::PositionType::Right, None);
        }
    }
    
    impl ObjectImpl for VideoPlayer {
        fn dispose(&self) {
            if let Ok(mut guard) = self.gstreamer_manager.lock() {
                if let Some(ref mut pipeline) = *guard {
                    pipeline.cleanup();
                } else {
                    eprintln!("Can't cleanup pipeline");
                }
            } else {
                eprintln!("Can't cleanup gstreamer_manager");
            }
        }

        fn constructed(&self) {
            self.setup_seek_bar();
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

        println!("created video player widget");
        widget
    }

    // fn start_updating_scale(scale: gtk::Scale, pipeline: gstreamer::Pipeline) {
    //     timeout_add_local(Duration::from_millis(500), move || {
    //         if let Some(position) = pipeline.query_position::<gstreamer::ClockTime>() {
    //             let pos_secs = position.seconds() as f64;
    //             scale.set_value(pos_secs);
    //         }
    //         glib::ControlFlow::Continue
    //     });
    // }

    fn start_updating_scale() {
        timeout_add_local(Duration::from_millis(500), move || {
            //println!("---------------------Updating Scale");
            glib::ControlFlow::Continue
        });
    }

    fn update_scale_value(&self, x: f64) {
        let imp = imp::VideoPlayer::from_obj(self);
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        if let Some(gstman) = gstman_weak.upgrade() {
            if let Ok(mut guard) = gstman.lock() {
                if let Some(ref mut pipeline) = *guard {
                    let percent = imp.seek_bar.value() / 100.0;
                    let position = pipeline.percent_to_position(percent).expect("Failed to get position");
                    println!("Position: {position}");
                    pipeline.seek_position(gstreamer::ClockTime::from_nseconds(position)).expect("Failed to seek position");
                }
            }
        }
    }

    fn connect_scale_drag_signals(&self, scale_box: &gtk::Box) {

        let gesture = gtk::GestureClick::new();

        gesture.connect_pressed(|_,_,x,y| {
            println!("---------------------Left click Begin at: x: {x}, y: {y}");
        });

        gesture.connect_released(glib::clone!(
            #[weak(rename_to = this)] self,
            move |_,_,x,y| {
                println!("---------------------Left click Ends at: x: {x}, y: {y}");
                this.update_scale_value(x);
            }
        ));

        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        scale_box.add_controller(gesture);
    }

    fn load_css() {
        let provider = CssProvider::new();
        match std::env::current_dir() {
            Ok(current_dir) => {
                let file = gio::File::for_path(current_dir.join("src\\widgets\\style.css"));
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

    pub fn setup_event_handlers(&self) {
        let imp = imp::VideoPlayer::from_obj(self);

        println!("Setting up buttons");
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.fchooser.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            #[weak(rename_to = text)] imp.text_view,
            #[weak(rename_to = pic)] imp.picture,
            #[weak(rename_to = win)] imp.obj().ancestor(gtk::ApplicationWindow::static_type()).unwrap(),
            move |_| {
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

                let gstman_weak_clone = gstman_weak.clone();
                dialog.run_async(move |obj, res| {
                    match res {
                        gtk::ResponseType::Accept => {
                            println!("Accepted");
                            if let Some(file) = obj.file() {
                                let from_str = gtk::gio::File::uri(&file);
                                println!("from_str {from_str}");
                                text.set_label(&from_str);
                                println!("File accepted: {}", from_str);
                                if let Some(gstman) = gstman_weak_clone.upgrade() {
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
                        }
                        _ => {
                            eprintln!("No file selected");
                        }
                    }
                    obj.destroy();
                });
            }
        ));

        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.previous_frame_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
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
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.play_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
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
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.stop_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
                if let Some(gstman) = gstman_weak.upgrade() {
                    if let Ok(mut guard) = gstman.lock() {
                        if let Some(ref mut pipeline) = *guard {
                            pipeline.stop_video();
                        } else {
                            eprintln!("No Video Pipeline available");
                        }
                    } else {
                        eprintln!("Failed to aquire lock on Video pipeline");
                    }
                }
            }
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.next_frame_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
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
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.test_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
                if let Some(gstman) = gstman_weak.upgrade() {
                    if let Ok(mut guard) = gstman.lock() {
                        if let Some(ref mut pipeline) = *guard {
                            println!("Testing");
                            pipeline.get_current_frame();
                        } else {
                            eprintln!("No Video Pipeline available");
                        }
                    } else {
                        eprintln!("Failed to aquire lock on Video pipeline");
                    }
                }
            }
        ));

        Self::start_updating_scale();

        //let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        //Self::connect_scale_signals(&imp.seek_bar, gstman_weak);
        Self::connect_scale_drag_signals(self,&imp.scale_parent);
        Self::load_css();


        
    }
}
