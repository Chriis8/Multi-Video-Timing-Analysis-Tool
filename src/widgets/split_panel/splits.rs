
use glib::object::ObjectExt;
use gtk::glib;
use gtk::subclass::prelude::*;
use imp::Segment;
use std::cell::RefCell;

mod imp {
    use std::u64;

    use glib::{value::ToValue, ParamSpecBuilderExt};

    use super::*;
    
    #[derive(Clone)]
    pub struct Segment {
        pub time: Option<u64>,
        pub duration: Option<u64>,
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
                    for i in 0..6 {
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
                        )
                    }
                    props
                });
                PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                n if n.starts_with("time-") => {
                    let i = n["time-".len()..].parse::<usize>().unwrap();
                    let val = self.segments.borrow().get(i).and_then(|v| v.time).unwrap_or(0);
                    val.to_value()
                }
                n if n.starts_with("duration-") => {
                    let i = n["duration-".len()..].parse::<usize>().unwrap();
                    let val = self.segments.borrow().get(i).and_then(|v| v.duration).unwrap_or(0);
                    val.to_value()
                }
                _ => unimplemented!()
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "name" => {
                    let val = value.get::<String>().unwrap();
                    *self.name.borrow_mut() = val;
                    self.notify(pspec);
                }
                n if n.starts_with("time-") => {
                    let i = n["time-".len()..].parse::<usize>().unwrap();
                    let segments = &mut self.segments.borrow_mut();
                    if i < segments.len() {
                        let raw = value.get::<u64>().unwrap_or_default();
                        segments[i].time = if raw == 0 { None } else { Some(raw) };
                        self.notify(pspec);
                    }
                }
                n if n.starts_with("duration-") => {
                    let i = n["duration-".len()..].parse::<usize>().unwrap();
                    let segments = &mut self.segments.borrow_mut();
                    if i < segments.len() {
                        let raw = value.get::<u64>().unwrap_or_default();
                        segments[i].duration = if raw == 0 { None } else { Some(raw) };
                        self.notify(pspec);
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
    
    pub fn get_time(&self, video_player_index: usize) -> Option<u64> {
        Some(self.property(&format!("time-{}", video_player_index)))
    }

    pub fn get_duration(&self, video_player_index: usize) -> Option<u64> {
        Some(self.property(&format!("duration-{}", video_player_index)))
    }

    pub fn get_segment_count(&self) -> usize {
        let imp = imp::VideoSegment::from_obj(self);
        imp.segments.borrow().len()
    }

    pub fn set_name(&self, new_name: String) {
        self.set_property("name", new_name);
    }

    pub fn add_empty_segment(&self) {
        let imp = imp::VideoSegment::from_obj(self);
        let new_segment = Segment {
            time: None,
            duration: None,
        };
        imp.segments.borrow_mut().push(new_segment);
    }

    pub fn set_time(&self, video_player_index: usize, time: u64) {
        self.set_property(&format!("time-{}", video_player_index), time);
    }

    pub fn set_duration(&self, video_player_index: usize, duration: u64) {
        self.set_property(&format!("duration-{}", video_player_index), duration);
    }
}
