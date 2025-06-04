
use glib::object::ObjectExt;
use gtk::glib;
use gtk::subclass::prelude::*;
use imp::Segment;
use std::cell::RefCell;
use crate::widgets::split_panel::timeentry::TimeEntry;
use std::collections::HashMap;
use std::u64;
use glib::{value::ToValue, ParamSpecBuilderExt};

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
        pub segments: RefCell<HashMap<String, Segment>>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoSegment {
        const NAME: &'static str = "VideoSegment";
        type Type = super::VideoSegment;
    }

    impl VideoSegment {
        fn notify_time_relative(&self, index: &str) {
            let prop_name = format!("relative-time-{}", index);
            self.obj().notify(&prop_name);
        }
    }

    impl ObjectImpl for VideoSegment {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: once_cell::sync::Lazy<Vec<glib::ParamSpec>> =
                once_cell::sync::Lazy::new(|| {
                    let mut props = vec![
                        glib::ParamSpecString::builder("name")
                            .nick("Name")
                            .blurb("Name of the segment")
                            .default_value(None)
                            .flags(glib::ParamFlags::READWRITE)
                            .build()
                    ];
                    for i in 1..7 {
                        props.push(glib::ParamSpecUInt64::builder(&format!("time-{}", i))
                            .nick(&format!("Time {}", i))
                            .blurb("Segment Time")
                            .minimum(0)
                            .maximum(u64::MAX)
                            .default_value(0)
                            .flags(glib::ParamFlags::READWRITE)
                            .build()
                        );
                        props.push(glib::ParamSpecUInt64::builder(&format!("duration-{}", i))
                            .nick(&format!("Duration {}", i))
                            .blurb("Duration Time")
                            .minimum(0)
                            .maximum(u64::MAX)
                            .default_value(0)
                            .flags(glib::ParamFlags::READWRITE)
                            .build()
                        );
                        props.push(glib::ParamSpecUInt64::builder(&format!("offset-{}", i))
                            .nick(&format!("Offset {}", i))
                            .blurb("Offset Time")
                            .minimum(0)
                            .maximum(u64::MAX)
                            .default_value(0)
                            .flags(glib::ParamFlags::READWRITE)
                            .build()
                        );
                        props.push(glib::ParamSpecUInt64::builder(&format!("relative-time-{}", i))
                            .nick(&format!("Time Relative {}", i))
                            .blurb("Time - Offset")
                            .minimum(0)
                            .maximum(u64::MAX)
                            .default_value(0)
                            .flags(glib::ParamFlags::READWRITE)
                            .build()
                        );
                    }
                    props
                });
                PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                n if n.starts_with("time-") => {
                    let id = &n["time-".len()..];
                    let segments_ref = self.segments.borrow();
                    let time_entry = &segments_ref.get(id).unwrap().time;
                    time_entry.get_time().to_value()
                }
                n if n.starts_with("duration-") => {
                    let id = &n["duration-".len()..];
                    let val = self.segments.borrow().get(id).and_then(|v| v.duration).unwrap_or(u64::MAX);
                    val.to_value()
                }
                n if n.starts_with("offset-") => {
                    let id = &n["offset-".len()..];
                    let segments_ref = self.segments.borrow();
                    let offset = &segments_ref.get(id).unwrap().offset;
                    offset.get_time().to_value()
                }
                n if n.starts_with("relative-time-") => {
                    let id = &n["relative-time-".len()..];
                    let segments_ref = self.segments.borrow();
                    let offset = segments_ref.get(id).unwrap().offset.get_time();
                    let time = segments_ref.get(id).unwrap().time.get_time();
                    if time == u64::MAX {
                        return time.to_value();
                    } else {
                        return time.saturating_sub(offset).to_value();
                    }
                }
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
                n if n.starts_with("time-") => {
                    let id = &n["time-".len()..];
                    let segments = &mut self.segments.borrow_mut();
                    if segments.contains_key(id) {
                        let raw = value.get::<u64>().unwrap_or_default();
                        segments.get(id).unwrap().time.set_time(raw);
                        self.notify(pspec);
                        self.notify(pspec);
                        self.notify_time_relative(id);
                    }
                }
                n if n.starts_with("duration-") => {
                    let id = &n["duration-".len()..];
                    let segments = &mut self.segments.borrow_mut();
                    if segments.contains_key(id) {
                        let raw = value.get::<u64>().unwrap_or_default();
                        if raw == u64::MAX {
                            segments.get_mut(id).unwrap().duration = None;
                        } else {
                            segments.get_mut(id).unwrap().duration = Some(raw);
                        }
                        self.notify(pspec);
                    }
                },
                n if n.starts_with("offset-") => {
                    let id = &n["offset-".len()..];
                    let segments = &mut self.segments.borrow_mut();
                    if segments.contains_key(id) {
                        let raw = value.get::<u64>().unwrap_or_default();
                        segments.get(id).unwrap().offset.set_time(raw);
                        self.notify(pspec);
                        self.notify_time_relative(id);
                    }
                }
                _ => {}
            }
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
        video_segment
    }

    pub fn get_name(&self) -> String {
        self.property::<String>("name")
    }
    
    pub fn get_time(&self, video_player_index: &str) -> Option<u64> {
        Some(self.property(&format!("time-{}", video_player_index)))
    }

    pub fn get_duration(&self, video_player_index: &str) -> Option<u64> {
        Some(self.property(&format!("duration-{}", video_player_index)))
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
        self.set_property(&format!("time-{}", video_player_id), time);
    }

    pub fn set_duration(&self, video_player_id: &str, duration: u64) {
        self.set_property(&format!("duration-{}", video_player_id), duration);
    }

    pub fn get_time_entry_copy(&self, video_player_id: &str) -> TimeEntry {
        let imp = imp::VideoSegment::from_obj(self);
        let time_entry = imp.segments.borrow().get(video_player_id).unwrap().time.clone();
        time_entry
    }

    pub fn set_offset(&self, video_player_id: &str, offset: u64) {
        println!("Setting {video_player_id} offset to: {offset}");
        self.set_property(&format!("offset-{}", video_player_id), offset);
    }

    pub fn get_offset(&self, video_player_id: &str) -> u64 {
        self.property(&format!("offset-{}", video_player_id))
    }

    pub fn reset_segment(&self, video_player_id: &str) {
        let imp = self.imp();
        self.set_time(video_player_id, u64::MAX);
        self.set_duration(video_player_id, u64::MAX);
        self.set_offset(video_player_id, 0);
        let new_segment = Segment {
            time: TimeEntry::new(u64::MAX),
            duration: None,
            offset: TimeEntry::new(0),
        };
        imp.segments.borrow_mut().insert(video_player_id.to_string(), new_segment);
    }

    pub fn get_keys(&self) -> Vec<String> {
        let imp = self.imp();
        let segment_borrow = imp.segments.borrow();
        let keys = segment_borrow.keys();
        return keys.cloned().collect::<Vec<String>>();
    }
}
