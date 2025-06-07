
use glib::object::ObjectExt;
use gtk::glib;
use gtk::subclass::prelude::*;
use std::cell::RefCell;
use crate::widgets::{split_panel::timeentry::TimeEntry, video_player_widget::video_player};
use std::collections::HashMap;
use std::u64;
use glib::{value::ToValue, ParamSpecBuilderExt};
use crate::widgets::split_panel::splits::VideoSegment;

mod imp {
    use super::*;
    
    #[derive(Default)]
    pub struct VideoSegmentProxy {
        pub segment: RefCell<Option<VideoSegment>>,
        pub video_player_id: RefCell<String>,
        pub field: RefCell<String>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for VideoSegmentProxy {
        const NAME: &'static str = "VideoSegmentProxy";
        type Type = super::VideoSegmentProxy;
    }

    impl VideoSegmentProxy {
        fn notify_value(&self) {
            self.obj().notify("value");
        }

        pub fn setup_notifies(&self) {
            let proxy_weak = self.downgrade();
            let seg_borrow = self.segment.borrow();
            let seg = seg_borrow.as_ref().unwrap();
            
            let relevant_signals = match self.field.borrow().as_str() {
                "time" => vec!["time"],
                "duration" => vec!["duration"],
                "offset" => vec!["offset"],
                "relative-time" => vec!["time", "offset"],
                _ => vec![],
            };
            
            for signal in relevant_signals {
                if let Some(proxy) = proxy_weak.upgrade() {
                    let video_player_id = self.video_player_id.borrow().clone();
                    seg.connect_local(signal, false, move |args| {
                        let emitted_id: &str = args[1].get().unwrap();
                        if emitted_id == video_player_id {
                            proxy.notify_value();
                        }
                        None
                    });
                }
            }

        }
    }

    impl ObjectImpl for VideoSegmentProxy {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: once_cell::sync::Lazy<Vec<glib::ParamSpec>> =
                once_cell::sync::Lazy::new(|| {
                    vec![
                        glib::ParamSpecObject::builder::<VideoSegment>("segment")
                            .readwrite()
                            .build(),
                        glib::ParamSpecString::builder("video-player-id")
                            .readwrite()
                            .build(),
                        glib::ParamSpecString::builder("field")
                            .readwrite()
                            .build(),
                        glib::ParamSpecUInt64::builder("value")
                            .readwrite()
                            .build(),
                    ]
                });
                PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "segment" => self.segment.borrow().to_value(),
                "video-player-id" => self.video_player_id.borrow().to_value(),
                "field" => self.field.borrow().to_value(),
                "value" => {
                    let segment_opt = self.segment.borrow();
                    let id = self.video_player_id.borrow();
                    let field = self.field.borrow();

                    if let Some(segment) = segment_opt.as_ref() {
                        let result = match field.as_str() {
                            "time" => Some(segment.get_time(&id)),
                            "duration" => segment.get_duration(&id),
                            "offset" => Some(segment.get_offset(&id)),
                            "relative-time" => {
                                let time = segment.get_time(&id);
                                if time == u64::MAX {
                                    Some(time)
                                } else {
                                    Some(time.saturating_sub(segment.get_offset(&id)))
                                }
                            },
                            _ => None,
                        };
                        return result.unwrap_or(u64::MAX).to_value();
                    }
                    u64::MAX.to_value()
                }
                _ => unimplemented!()
            }
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            println!("Set property id: {_id}, value: {value:?}, pspec: {pspec:?}");
            match pspec.name() {
                "segment" => {
                    let segment = value.get::<VideoSegment>().ok();
                    *self.segment.borrow_mut() = segment;
                    self.notify_value();
                }
                "video-player-id" => {
                    let val = value.get::<String>().unwrap_or_default();
                    *self.video_player_id.borrow_mut() = val;
                    self.notify_value();
                }
                "field" => {
                    let val = value.get::<String>().unwrap_or_default();
                    *self.field.borrow_mut() = val;
                    self.notify_value();
                }
                _ => {}
            }
        }

        
    }
}

glib::wrapper! {
    pub struct VideoSegmentProxy(ObjectSubclass<imp::VideoSegmentProxy>)
    @implements gtk::Buildable;
}

// Video Segment:
// Name: Name of the segment
// Segment: time and duration of the split
impl VideoSegmentProxy {
    pub fn new(segment: &VideoSegment, video_player_id: &str, field: &str) -> Self {
        let object = glib::Object::builder()
            .property("segment", segment)
            .property("video-player-id", video_player_id)
            .property("field", field)
            .build();
        let imp = imp::VideoSegmentProxy::from_obj(&object);
        imp.setup_notifies();
        object
    }

}
