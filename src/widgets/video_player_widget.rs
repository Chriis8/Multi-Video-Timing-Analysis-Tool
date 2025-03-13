use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/videoplayer/video_player.ui")]
    pub struct VideoPlayer {
        #[template_child]
        pub vbox: TemplateChild<gtk::Box>,

        #[template_child]
        pub hboxtop: TemplateChild<gtk::Box>,

        #[template_child]
        pub fchooser: TemplateChild<gtk::Button>,

        #[template_child]
        pub text_view: TemplateChild<gtk::Label>,

        #[template_child]
        pub picture: TemplateChild<gtk::Picture>,

        #[template_child]
        pub label: TemplateChild<gtk::Label>,

        #[template_child]
        pub hbox: TemplateChild<gtk::Box>,

        #[template_child]
        pub previous_frame_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub stop_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub next_frame_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub test_button: TemplateChild<gtk::Button>
    }

    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoPlayer {
        const NAME: &'static str = "VideoPlayer";
        type Type = super::VideoPlayer;
        type ParentType = gtk::Box; // or gtk::Box if preferred

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
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
        eprint!("created video player widget");
        glib::Object::new::<Self>()
    }
}