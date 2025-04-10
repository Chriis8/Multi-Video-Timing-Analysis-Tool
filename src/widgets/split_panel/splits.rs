
use gtk::glib;
use gtk::subclass::prelude::*;
use imp::Segment;
use std::cell::RefCell;

mod imp {
    use super::*;
    
    #[derive(Clone)]
    pub struct Segment {
        pub time: u64,
        pub duration: u64,
    }

    #[derive(Default)]
    pub struct VideoSegment {
        pub name: RefCell<String>,
        pub segments: RefCell<Vec<Segment>>,
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
    pub fn new(name: &str) -> Self {
        let segment: Self = glib::Object::new::<Self>();
        let imp = imp::VideoSegment::from_obj(&segment);
        *imp.name.borrow_mut() = name.to_string();
        segment
    }

    pub fn get_name(&self) -> String {
        let imp = imp::VideoSegment::from_obj(self);
        imp.name.borrow().clone()
    }
    
    pub fn get_segment(&self, video_number: usize) -> Option<Segment> {
        let imp = imp::VideoSegment::from_obj(self);
        imp.segments.borrow().get(video_number).cloned()
    }

    pub fn count(&self) -> usize {
        let imp = imp::VideoSegment::from_obj(self);
        imp.segments.borrow().len()
    }

    pub fn get_segments(&self, index: usize) -> (u64, u64) {
        let imp = imp::VideoSegment::from_obj(self);
        (imp.segments.borrow()[index].time, imp.segments.borrow()[index].duration)
    }

    pub fn set_name(&self, new_name: String) {
        let imp = imp::VideoSegment::from_obj(self);
        imp.name.replace(new_name);
    }

    pub fn add_segment(&self, time: u64, duration: u64) {
        let imp = imp::VideoSegment::from_obj(self);
        let new_segment = Segment {
            time: time,
            duration: duration,
        };
        imp.segments.borrow_mut().push(new_segment);
    }

    pub fn set_segment(&self, index: usize, time: u64, duration: u64) {
        let imp = imp::VideoSegment::from_obj(self);
        let seg = &mut imp.segments.borrow_mut()[index];
        seg.time = time;
        seg.duration = duration;
    }
}
