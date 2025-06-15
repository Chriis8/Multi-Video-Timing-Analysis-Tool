
use glib::object::ObjectExt;
use gtk::glib;
use gtk::subclass::prelude::*;
use glib::subclass::Signal;
use imp::Segment;
use std::cell::RefCell;
use crate::{helpers::data::get_next_id, widgets::split_panel::timeentry::TimeEntry};
use std::collections::HashMap;
use std::u64;
use glib::{value::ToValue, ParamSpecBuilderExt};
use once_cell::sync::Lazy;
use gtk::prelude::*;

mod imp {
    use super::*;
    
    #[derive(Clone)]
    pub struct Segment {
        pub time: TimeEntry,
        pub duration: Option<u64>,
        pub offset: TimeEntry,
    }

    #[derive(Default)]
    pub struct VideoSegment {
        pub name: RefCell<String>,
        pub id: RefCell<String>,
        pub segments: RefCell<HashMap<String, Segment>>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoSegment {
        const NAME: &'static str = "VideoSegment";
        type Type = super::VideoSegment;
    }

    impl VideoSegment {
    }

    impl ObjectImpl for VideoSegment {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: once_cell::sync::Lazy<Vec<glib::ParamSpec>> =
                once_cell::sync::Lazy::new(|| {
                    vec![
                        glib::ParamSpecString::builder("name")
                            .nick("Name")
                            .blurb("Name of the segment")
                            .default_value(None)
                            .flags(glib::ParamFlags::READWRITE)
                            .build()
                    ]
                });
                PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                _ => unimplemented!()
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            println!("Set property id: {_id}, value: {value:?}, pspec: {pspec:?}");
            match pspec.name() {
                "name" => {
                    let val = value.get::<String>().unwrap();
                    *self.name.borrow_mut() = val;
                    self.notify(pspec);
                }
                _ => {}
            }
        }

        fn signals() -> &'static [Signal] {
            // Setup split button signal
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("time")
                        .flags(glib::SignalFlags::RUN_LAST)
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("duration")
                        .flags(glib::SignalFlags::RUN_LAST)
                        .param_types([String::static_type()])
                        .build(),
                    Signal::builder("offset")
                        .flags(glib::SignalFlags::RUN_LAST)
                        .param_types([String::static_type()])
                        .build(),
                    ]
            });
            SIGNALS.as_ref()
        }
    }
}

glib::wrapper! {
    pub struct VideoSegment(ObjectSubclass<imp::VideoSegment>)
    @implements gtk::Buildable;
}

// Video Segment:
// Name: Name of the segment
// Segment: time and duration of the split
impl VideoSegment {
    pub fn new(name: &str) -> Self {
        let video_segment: Self = glib::Object::new::<Self>();
        let imp = imp::VideoSegment::from_obj(&video_segment);
        *imp.name.borrow_mut() = name.to_string();
        *imp.id.borrow_mut() = get_next_id().to_string();
        video_segment
    }

    pub fn get_name(&self) -> String {
        self.property::<String>("name")
    }
    
    pub fn get_time(&self, video_player_index: &str) -> u64 {
        let imp = self.imp();
        imp.segments.borrow().get(video_player_index).unwrap().time.get_time()
        //Some(self.property(&format!("time-{}", video_player_index)))
    }

    pub fn get_duration(&self, video_player_index: &str) -> Option<u64> {
        let imp = self.imp();
        imp.segments.borrow().get(video_player_index).unwrap().duration
        //Some(self.property(&format!("duration-{}", video_player_index)))
    }

    pub fn get_segment_count(&self) -> usize {
        let imp = imp::VideoSegment::from_obj(self);
        imp.segments.borrow().len()
    }

    pub fn set_name(&self, new_name: String) {
        self.set_property("name", new_name);
    }

    pub fn add_empty_segment(&self, id: &str) -> Segment {
        let imp = imp::VideoSegment::from_obj(self);
        let new_segment = Segment {
            time: TimeEntry::new(u64::MAX),
            duration: None,
            offset: TimeEntry::new(0),
        };
        imp.segments.borrow_mut().insert(id.to_string(), new_segment.clone());
        new_segment
    }

    pub fn set_time(&self, video_player_id: &str, time: u64) {
        println!("Setting times to {time}");
        let imp = self.imp();
        imp.segments.borrow().get(video_player_id).unwrap().time.set_time(time);
        let id: &dyn ToValue = &video_player_id.to_string();
        self.emit_by_name::<()>("time", &[id]);
        //self.set_property(&format!("time-{}", video_player_id), time);
    }

    pub fn set_duration(&self, video_player_id: &str, duration: u64) {
        println!("Setting duration to {duration}");
        let imp = self.imp();
        imp.segments.borrow_mut().get_mut(video_player_id).unwrap().duration = Some(duration);
        let id: &dyn ToValue = &video_player_id.to_string();
        self.emit_by_name::<()>("duration", &[id]);
        //self.set_property(&format!("duration-{}", video_player_id), duration);
    }

    pub fn get_time_entry_copy(&self, video_player_id: &str) -> TimeEntry {
        let imp = imp::VideoSegment::from_obj(self);
        let time_entry = imp.segments.borrow().get(video_player_id).unwrap().time.clone();
        time_entry
    }

    pub fn set_offset(&self, video_player_id: &str, offset: u64) {
        println!("Setting {video_player_id} offset to: {offset}");
        let imp = self.imp();
        imp.segments.borrow().get(video_player_id).unwrap().offset.set_time(offset);
        let id: &dyn ToValue = &video_player_id.to_string();
        self.emit_by_name::<()>("offset", &[id]);
        //self.set_property(&format!("offset-{}", video_player_id), offset);
    }

    pub fn get_offset(&self, video_player_id: &str) -> u64 {
        let imp = self.imp();
        imp.segments.borrow().get(video_player_id).unwrap().offset.get_time()
        //Some(self.property(&format!("offset-{}", video_player_id)))
    }

    pub fn get_keys(&self) -> Vec<String> {
        let imp = self.imp();
        let segment_borrow = imp.segments.borrow();
        let keys = segment_borrow.keys();
        return keys.cloned().collect::<Vec<String>>();
    }

    pub fn get_segment_id(&self) -> String {
        let imp = self.imp();
        imp.id.borrow().to_string()
    }

    pub fn remove_segment(&self, video_player_id: &str) {
        let imp = self.imp();
        let mut segment_borrow = imp.segments.borrow_mut();
        segment_borrow.remove(video_player_id);
    }
}
