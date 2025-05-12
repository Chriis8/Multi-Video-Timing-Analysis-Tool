use glib::{object::ObjectExt, subclass::types::ObjectSubclassExt};

mod imp {
    use gtk::glib;
    use gtk::subclass::prelude::*;
    use std::u64;
    use std::cell::RefCell;
    use glib::{ParamSpecBuilderExt, ParamSpecUInt64, value::ToValue};

    #[derive(Default)]
    pub struct TimeEntry {
        pub time: RefCell<u64>,
        pub old_time: RefCell<u64>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for TimeEntry {
        const NAME: &'static str = "TimeEntry";
        type Type = super::TimeEntry;
    }

    impl ObjectImpl for TimeEntry{
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: once_cell::sync::Lazy<Vec<glib::ParamSpec>> =
                once_cell::sync::Lazy::new(|| {
                    vec![ParamSpecUInt64::builder("time")
                        .nick("Time")
                        .blurb("Time in nanoseconds")
                        .minimum(0)
                        .maximum(u64::MAX)
                        .flags(glib::ParamFlags::READWRITE)
                        .build(),
                        ParamSpecUInt64::builder("old-time")
                        .nick("Old Time")
                        .blurb("Old time in nanoseconds")
                        .minimum(0)
                        .maximum(u64::MAX)
                        .flags(glib::ParamFlags::READWRITE)
                        .build()]
                });
                PROPERTIES.as_ref()
        }

        fn property(&self, id: usize, _pspec: &glib::ParamSpec) -> glib::Value {
            match id {
                1 => {
                    self.time.borrow().to_value()
                },
                2 => {
                    self.old_time.borrow().to_value()
                }
                _ => unimplemented!(),
            }
        }

        fn set_property(&self, id: usize, value: &glib::Value, _pspec: &glib::ParamSpec) {
            match id {
                1 => {
                    let val = value.get::<u64>().unwrap();
                    let old_time = *self.time.borrow();
                    *self.old_time.borrow_mut() = old_time;
                    *self.time.borrow_mut() = val;
                    println!("Set Value: {val}, Old Value: {old_time}")
                },
                2 => {

                },
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct TimeEntry(ObjectSubclass<imp::TimeEntry>)
    @implements gtk::Buildable;
}

impl TimeEntry {
    pub fn new(time: u64) -> Self {
        let time_entry = glib::Object::new::<Self>();
        time_entry.set_property("time", time);
        time_entry.set_property("old-time", time);
        time_entry
    }

    pub fn get_time(&self) -> u64 {
        self.property("time")
    }

    pub fn set_time(&self, new_time: u64) {
        self.set_property("time", new_time);
    }

    pub fn get_old_time(&self) -> u64 {
        self.property("old-time")
    }

    // fn set_old_time(&self, time: u64) {
    //     self.set_property("old-time", time);
    // } 
}