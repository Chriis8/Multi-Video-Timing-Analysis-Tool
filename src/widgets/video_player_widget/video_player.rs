use glib::timeout_add_local;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::CssProvider;
use gtk::gdk::Display;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use crate::video_pipeline::VideoPipeline;
use crate::widgets::split_panel::timeentry::TimeEntry;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use once_cell::sync::Lazy;
use crate::widgets::seek_bar::seek_bar::SeekBar;

mod imp {
    use gtk::{Box, Button, Label, Picture};
    use glib::subclass::Signal;


    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/videoplayer/vplayer.ui")]
    pub struct VideoPlayer {
        pub gstreamer_manager: Arc<Mutex<VideoPipeline>>,

        pub timeout_id: Rc<RefCell<Option<glib::SourceId>>>,

        pub continue_timeout: RefCell<bool>,
        
        pub is_dragging: Rc<Cell<bool>>,
        
        pub id: Cell<u32>,
        
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

        // #[template_child]
        // pub scale_parent: TemplateChild<Box>,

        #[template_child]
        pub seek_bar: TemplateChild<SeekBar>,

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
        pub split_button: TemplateChild<Button>,

        #[template_child]
        pub set_start_time_button: TemplateChild<Button>,
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
            let scale = self.seek_bar.get_scale();
            scale.set_adjustment(&adjustment);
        }

        fn set_controls(&self, status: bool) {
            self.next_frame_button.set_sensitive(status);
            self.previous_frame_button.set_sensitive(status);
            self.play_button.set_sensitive(status);
            self.stop_button.set_sensitive(status);
            self.set_start_time_button.set_sensitive(status);
            self.split_button.set_sensitive(status);
        }

        fn set_scale_interation(&self, status: bool) {
            self.seek_bar.set_sensitive(status);
        }
    }
    
    impl ObjectImpl for VideoPlayer {
        fn dispose(&self) {
            if let Ok(mut pipeline) = self.gstreamer_manager.lock() {
                println!("pipeline cleanup");
                *self.continue_timeout.borrow_mut() = false;
                pipeline.cleanup();
            } else {
                eprintln!("Can't cleanup gstreamer_manager");
            }
        }

        fn constructed(&self) {
            self.setup_seek_bar();
            self.set_controls(false);
            self.set_scale_interation(false);
        }

        fn signals() -> &'static [Signal] {
            // Setup split button signal
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("split-button-clicked")
                    .flags(glib::SignalFlags::RUN_LAST)
                    .param_types([u32::static_type(), u64::static_type()])
                    .build(),
                    Signal::builder("timeline-length-acquired")
                    .flags(glib::SignalFlags::RUN_LAST)
                    .param_types([u64::static_type()])
                    .build(),
                    Signal::builder("set-start-button-clicked")
                    .flags(glib::SignalFlags::RUN_LAST)
                    .param_types([u32::static_type(), u64::static_type()])
                    .build(),
                    Signal::builder("seek-bar-pressed")
                    .flags(glib::SignalFlags::RUN_LAST)
                    .build(),
                    Signal::builder("pipeline-built")
                    .flags(glib::SignalFlags::RUN_LAST)
                    .build(),
                    ]
                });
            SIGNALS.as_ref()
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

// Video Player:
// Custom widget that includes the open file navigation, main video, media control, split button
impl VideoPlayer {
    // Creates new video player widget
    pub fn new(id: u32) -> Self {
        let widget: Self = glib::Object::new::<Self>();
        
        let imp = imp::VideoPlayer::from_obj(&widget);
        
        *imp.gstreamer_manager.lock().unwrap() = VideoPipeline::new();

        *imp.continue_timeout.borrow_mut() = false;
        imp.seek_bar.set_auto_timeline_length_handling(false);

        imp.seek_bar.update_marks_on_width_change_timeout();
        imp.id.set(id);

        println!("created video player widget");
        widget
    }

    // Controls automatic seek bar movement while video is playing
    fn start_updating_scale(&self, scale: &gtk::Scale) {
        println!("Starting to update the seek bar");
        let imp = imp::VideoPlayer::from_obj(self);
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        let seek_bar_clone = scale.clone();
        let is_dragging_clone = imp.is_dragging.clone();
        *imp.continue_timeout.borrow_mut() = true;
        let to_continue = imp.continue_timeout.clone();
        // Sets up timeout to update the seekbar every 100 milliseconds
        let source_id = timeout_add_local(Duration::from_millis(100), move || {
            if !*to_continue.borrow() {
                println!("breaking update scale timeout");
                return glib::ControlFlow::Break
            }
            // Skips update if user is moving the seek bar
            if is_dragging_clone.get() {
                println!("Dragging, skipping update scale");
                return glib::ControlFlow::Continue
            }
            // Updates the seek bar based on the videos position
            if let Some(gstman) = gstman_weak.upgrade() {
                if let Ok(pipeline) = gstman.lock() {
                    if let Ok(new_value) = pipeline.position_to_percent() {
                        seek_bar_clone.set_value(new_value);
                    } else {
                        eprintln!("Pipeline not ready");
                    }
                }
            }
            glib::ControlFlow::Continue
        });
        *imp.timeout_id.borrow_mut() = Some(source_id);
    }

    // Updates video position from seek bar position
    fn update_scale_value(&self) {
        let imp = imp::VideoPlayer::from_obj(self);
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        if let Some(gstman) = gstman_weak.upgrade() {
            if let Ok(mut pipeline) = gstman.lock() {
                // gets seek bar progress 0.0 - 1.0
                let percent = imp.seek_bar.get_scale().value() / 100.0;
                // Gets precentage time in nanoseconds of the total videos duration
                let position = pipeline.percent_to_position(percent).expect("Failed to get position");
                println!("Position: {position}");
                // Updates the video players position from acquired position
                pipeline.seek_position(gstreamer::ClockTime::from_nseconds(position)).expect("Failed to seek position");
            }
        }
    }

    // Connects user control
    fn connect_scale_drag_signals(&self, scale_box: &crate::SeekBar) {

        let imp = imp::VideoPlayer::from_obj(self);

        let gesture = gtk::GestureClick::new();
        gesture.connect_pressed(glib::clone!(
            #[weak(rename_to = this)] self,
            #[weak(rename_to = is_dragging_weak)] imp.is_dragging,
            move |_,_,_x,_y| {
                //println!("---------------------Left click Begin at: x: {x}, y: {y}");
                is_dragging_weak.set(true);
                println!("emiiiittttttting seek-bar-pressed");
                this.emit_by_name::<()>("seek-bar-pressed", &[]);
            }
        ));

        gesture.connect_released(glib::clone!(
            #[weak(rename_to = this)] self,
            #[weak(rename_to = is_dragging_weak)] imp.is_dragging,
            move |_,_,_x,_y| {
                //println!("---------------------Left click Ends at: x: {x}, y: {y}");
                this.update_scale_value();
                is_dragging_weak.set(false);                
            }
        ));

        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        scale_box.add_controller(gesture);
    }

    fn load_css() {
        let provider = CssProvider::new();
        match std::env::current_dir() {
            Ok(current_dir) => {
                let file = gio::File::for_path(current_dir.join("src\\widgets\\video_player_widget\\style.css"));
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

    pub fn setup_event_handlers(&self, window: gtk::ApplicationWindow) {
        let imp = imp::VideoPlayer::from_obj(self);

        println!("Setting up buttons");
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        // File Chooser / Open file button
        imp.fchooser.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            #[weak(rename_to = text)] imp.text_view,
            #[weak(rename_to = pic)] imp.picture,
            #[weak(rename_to = win)] window,
            #[weak(rename_to = this)] self,
            #[weak(rename_to = seekbar)] imp.seek_bar,
            move |_| {
                this.set_controls(false);
                this.set_scale_interation(false);
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
                                    if let Ok(mut pipeline) = gstman.lock() {
                                        pipeline.reset();
                                        pipeline.build_pipeline(Some(&text.label().to_string()));
                                        let paintable = pipeline.get_paintable();
                                        pic.set_paintable(Some(&paintable));
                                        let scale = seekbar.get_scale();
                                        this.start_updating_scale(&scale);
                                        let timeline_length = pipeline.get_length().unwrap();
                                        seekbar.set_timeline_length(timeline_length);
                                        let nanos: &dyn ToValue = &timeline_length;
                                        this.emit_by_name::<()>("timeline-length-acquired", &[nanos]);
                                        this.set_controls(true);
                                        this.set_scale_interation(true);
                                    } else {
                                        eprintln!("Failed to aquire lock on Video pipeline");
                                    }
                                    this.emit_by_name::<()>("pipeline-built", &[]);
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
        // Moves video one frame backward
        imp.previous_frame_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
                if let Some(gstman) = gstman_weak.upgrade() {
                    if let Ok(mut pipeline) = gstman.lock() {
                        pipeline.frame_backward();
                    } else {
                        eprintln!("Failed to aquire lock on Video pipeline");
                    }
                }
            }
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        // Set video to playing state
        imp.play_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
                if let Some(gstman) = gstman_weak.upgrade() {
                    if let Ok(mut pipeline) = gstman.lock() {
                        pipeline.play_video();
                    } else {
                        eprintln!("Failed to aquire lock on Video pipeline");
                    }
                }
            }
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        // Closes video file
        imp.stop_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            #[weak(rename_to = timeout_id)] imp.timeout_id,
            move |_| {
                if let Some(id) = timeout_id.borrow_mut().take() {
                    id.remove();
                }     
                if let Some(gstman) = gstman_weak.upgrade() {
                    if let Ok(mut pipeline) = gstman.lock() {
                        pipeline.stop_video();
                    } else {
                        eprintln!("Failed to aquire lock on Video pipeline");
                    }
                }
            }
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        // Moves video one frame forward
        imp.next_frame_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            move |_| {
                if let Some(gstman) = gstman_weak.upgrade() {
                    if let Ok(mut pipeline) = gstman.lock() {
                        pipeline.frame_forward();
                    } else {
                        eprintln!("Failed to aquire lock on Video pipeline");
                    }
                }
            }
        ));
        
        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        // Split button: Splits the video at the current time for the currently selected segment.
        imp.split_button.connect_clicked(glib::clone!(
            #[weak(rename_to = this)] self,
            #[strong] gstman_weak,
            #[weak] imp,
            move |_| {
                let gstman = match gstman_weak.upgrade() {
                    Some(val) => val, None => return,
                };
                let mut pipeline = match gstman.lock() {
                    Ok(val) => val, Err(_) => return,
                };
                
                let pos = match pipeline.get_position() {
                    Some(time) => time,
                    None => {
                        eprintln!("Failed to get position trying to set split time");
                        return;
                    }
                };
                let nanos: &dyn ToValue = &pos.nseconds();
                let id: &dyn ToValue = &imp.id.get();
                this.emit_by_name::<()>("split_button_clicked", &[id, nanos]);
            }
        ));

        let gstman_weak = Arc::downgrade(&imp.gstreamer_manager);
        imp.set_start_time_button.connect_clicked(glib::clone!(
            #[strong] gstman_weak,
            #[weak(rename_to = this)] self,
            #[weak] imp,
            move |_| {
                let gstman = match gstman_weak.upgrade() {
                    Some(val) => val, None => return,
                };
                let mut pipeline = match gstman.lock() {
                    Ok(val) => val, Err(_) => return,
                };
                let pos = match pipeline.get_position() {
                    Some(time) => time,
                    None => {
                        eprintln!("Failed to get position trying to set the start time offset");
                        return;
                    }
                };
                let nanos: &dyn ToValue = &pos.nseconds();
                let id: &dyn ToValue = &imp.id.get();
                this.emit_by_name::<()>("set-start-button-clicked", &[id, nanos]);
            }
        ));

        Self::connect_scale_drag_signals(self,&imp.seek_bar);
        Self::load_css();
    }

    // // Gets split button - idk if this is used
    // pub fn split_button(&self) -> gtk::Button {
    //     let imp = imp::VideoPlayer::from_obj(self);
    //     imp.split_button.clone()
    // }

    // Gets the video players pipeline
    pub fn pipeline(&self) -> Weak<Mutex<VideoPipeline>> {
        let imp = imp::VideoPlayer::from_obj(self);
        Arc::downgrade(&imp.gstreamer_manager)
    }

    pub fn connect_time_to_seekbar(&self, id: String, time_entry: TimeEntry, color: &str) {
        let imp = imp::VideoPlayer::from_obj(self);
        imp.seek_bar.add_mark(id, time_entry, color, TimeEntry::new(0));
    }

    pub fn set_controls(&self, status: bool) {
        let imp = self.imp();
        imp.next_frame_button.set_sensitive(status);
        imp.previous_frame_button.set_sensitive(status);
        imp.play_button.set_sensitive(status);
        imp.stop_button.set_sensitive(status);
        imp.set_start_time_button.set_sensitive(status);
        imp.split_button.set_sensitive(status);
    }

    pub fn set_scale_interation(&self, status: bool) {
        let imp = self.imp();
        imp.seek_bar.set_sensitive(status);
    }

    pub fn get_seek_bar(&self) -> Option<SeekBar> {
        let imp = self.imp();
        let sb = imp.seek_bar.get();
        return Some(sb);
    }
}
