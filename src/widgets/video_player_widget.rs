use gtk::glib;
//use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

mod imp {
    use gtk::{Button, Box, Label, Picture, template_callbacks};
    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/videoplayer/video_player.ui")]
    pub struct VideoPlayer {
        pub gstreamer_manager: std::sync::Arc<std::sync::Mutex<Option<crate::video_pipeline::VideoPipeline>>>,        
        
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
    
    #[template_callbacks]
    impl VideoPlayer {
        #[template_callback]
        fn handle_fchooser_clicked(&self, _button: &Button) {
            eprintln!("fchooser clicked");
        }
    
        #[template_callback]
        fn handle_previous_frame_clicked(&self, _button: &Button) {
            eprintln!("preivous frame clicked");
        }
    
        #[template_callback]
        fn handle_play_clicked(&self, _button: &Button) {
            eprintln!("play clicked");
        }
    
        #[template_callback]
        fn handle_stop_clicked(&self, _button: &Button) {
            eprintln!("stop clicked");
        }
    
        #[template_callback]
        fn handle_next_frame_clicked(&self, _button: &Button) {
            eprintln!("next frame clicked");
        }
    
        #[template_callback]
        fn handle_test_clicked(&self, _button: &Button) {
            eprintln!("test clicked");
        }
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoPlayer {
        const NAME: &'static str = "VideoPlayer";
        type Type = super::VideoPlayer;
        type ParentType = gtk::Box;
        
        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::bind_template_callbacks(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    
    impl ObjectImpl for VideoPlayer {}
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
            *pipeline = Some(crate::video_pipeline::VideoPipeline::new());
        }
        
        eprint!("created video player widget");
        widget
    }
    
}
