
use gtk::glib;
use gtk::subclass::prelude::*;
use std::cell::RefCell;

mod imp {


    use super::*;
    
    #[derive(Default)]
    pub struct VideoSegment {
        pub name: RefCell<String>,
        pub time: RefCell<u64>,
        pub duration: RefCell<u64>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoSegment {
        const NAME: &'static str = "VideoSegment";
        type Type = super::VideoSegment;
    }
    
    impl ObjectImpl for VideoSegment {}
    
}

glib::wrapper! {
    pub struct VideoSegment(ObjectSubclass<imp::VideoSegment>)
    @implements gtk::Buildable;
}

impl VideoSegment {
    pub fn new(name: &str, time: u64, duration: u64) -> Self {
        let segment: Self = glib::Object::new::<Self>();
        let imp = imp::VideoSegment::from_obj(&segment);

        *imp.name.borrow_mut() = name.to_string();
        *imp.time.borrow_mut() = time;
        *imp.duration.borrow_mut() = duration;

        segment
    }

    pub fn get_name(&self) -> String {
        let imp = imp::VideoSegment::from_obj(self);
        imp.name.borrow().clone()
    }
    
    pub fn get_time(&self) -> u64 {
        let imp = imp::VideoSegment::from_obj(self);
        *imp.time.borrow()
    }

    pub fn get_duration(&self) -> u64 {
        let imp = imp::VideoSegment::from_obj(self);
        *imp.duration.borrow()
    }
}
