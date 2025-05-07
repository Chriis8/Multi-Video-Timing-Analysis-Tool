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
                        .build()]
                });
                PROPERTIES.as_ref()
        }

        fn property(&self, id: usize, _pspec: &glib::ParamSpec) -> glib::Value {
            if id == 1 {
                self.time.borrow().to_value()
            } else {
                panic!("Invalid property ID {}", id);
            }
        }

        fn set_property(&self, id: usize, value: &glib::Value, _pspec: &glib::ParamSpec) {
            if id == 1 {
                let val = value.get::<u64>().unwrap();
                let time= &mut *self.time.borrow_mut();
                *time = val;
                println!("Set Value: {val}");
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
        time_entry
    }

    pub fn get_time(&self) -> u64 {
        self.property("time")
    }

    pub fn set_time(&self, new_time: u64) {
        self.set_property("time", new_time);
    }
}