use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)]
    #[template(file = "video_player.ui")]
    pub struct VideoPlayer {
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,

    }

    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoPlayer {
        const NAME: &'static str = "VideoPlayer";
        type Type = super::VideoPlayer;
        type ParentType = gtk::Widget; // or gtk::Box if preferred

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VideoPlayer {}
    impl WidgetImpl for VideoPlayer {}
}

glib::wrapper! {
    pub struct VideoPlayer(ObjectSubclass<imp::VideoPlayer>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}


impl VideoPlayer {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}