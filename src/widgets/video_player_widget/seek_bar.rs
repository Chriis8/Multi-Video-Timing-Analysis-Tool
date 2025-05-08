use gtk::prelude::*;
use gtk::glib;
use gtk::subclass::fixed;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::{Label, Overlay, Box, Scale, TemplateChild, Fixed};
use std::{cell::RefCell, collections::HashMap};
use std::rc::Rc;
use crate::widgets::split_panel::timeentry::TimeEntry;

mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)] 
    #[template(resource = "/seekbar/seekbar.ui")]
    pub struct SeekBar {
        pub marks: RefCell<HashMap<String, (Rc<TimeEntry>, gtk::Widget)>>,
        pub timeline_length: Rc<RefCell<u64>>,

        #[template_child]
        pub scale: TemplateChild<Scale>,

        #[template_child]
        pub fixed: TemplateChild<Fixed>,

        #[template_child]
        pub overlay: TemplateChild<Overlay>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for SeekBar {
        const NAME: &'static str = "SeekBar";
        type Type = super::SeekBar;
        type ParentType = Box;
        
        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    
    impl ObjectImpl for SeekBar {
        fn dispose(&self) {
        }
    }
    impl WidgetImpl for SeekBar {}
    impl BoxImpl for SeekBar {}
}

glib::wrapper! {
    pub struct SeekBar(ObjectSubclass<imp::SeekBar>)
    @extends gtk::Widget,
    @implements gtk::Buildable;
}

impl SeekBar {
    pub fn new(timeline_length: u64) -> Self {
        let widget: Self = glib::Object::new::<Self>();
        let imp = imp::SeekBar::from_obj(&widget);
        *imp.timeline_length.borrow_mut() = timeline_length;

        widget
    }

    pub fn add_mark(&self, id: String, time_entry: Rc<TimeEntry>, color: &str) {
        let imp = imp::SeekBar::from_obj(self);
        
        let mark = Label::new(None);
        mark.set_markup(&format!("<span foreground='{color}'>^</span>"));
        mark.set_halign(gtk::Align::Start);
        mark.set_valign(gtk::Align::Center);

        imp.fixed.put(&mark, 0.0, 0.0);
        self.update_mark_position(&mark.clone().upcast(), &time_entry);

        time_entry.connect_notify_local(Some("time"), glib::clone!(
            #[strong(rename_to = fixed_overlay)] imp.fixed.clone(),
            #[strong(rename_to = mark_clone)] mark.clone(),
            #[strong(rename_to = timeline_length)] imp.timeline_length,
            move |time_entry, _| {
                let time = time_entry.get_time();
                let width = fixed_overlay.width();
                if *timeline_length.borrow() == 0 {
                    eprintln!("Timeline_length is still 0");
                    return;
                }
                if time == u64::MAX {
                    mark_clone.set_visible(false);
                    let percent = 0.0;
                    let x = percent * width as f64;
                    fixed_overlay.move_(&mark_clone, x, 25.0);
                } else {
                    mark_clone.set_visible(true);
                    let percent = time as f64 / *timeline_length.borrow() as f64;
                    let x = percent * width as f64;
                    fixed_overlay.move_(&mark_clone, x, 25.0);
                }
            }
        ));
        
        mark.set_visible(true);
        //imp.overlay.add_overlay_pass_through(&mark, true);
        
        imp.marks.borrow_mut().insert(id.clone(), (time_entry, mark.clone().upcast()));
    }

    pub fn remove_mark(&self, id: &str) {
        let imp = imp::SeekBar::from_obj(self);
        if let Some((_, widget)) = imp.marks.borrow_mut().remove(id) {
            widget.unparent();
        }
    }

    pub fn update_mark_time(&self, id: &str, new_time: u64) {
        let imp = imp::SeekBar::from_obj(self);
        if let Some((time_entry, _)) = imp.marks.borrow().get(id) {
            time_entry.set_time(new_time);
        }
    }

    fn update_mark_position(&self, widget: &gtk::Widget, time_entry: &TimeEntry) {
        let time = time_entry.get_time();
        let imp = imp::SeekBar::from_obj(self);
        if *imp.timeline_length.borrow() == 0 {
            return;
        }
        if time == u64::MAX {
            widget.set_visible(false);
            let x_pos = self.time_to_position(0);
            imp.fixed.move_(widget, x_pos, 25.0);
        } else {
            widget.set_visible(true);
            let x_pos = self.time_to_position(time);
            imp.fixed.move_(widget, x_pos, 25.0);
        }
    }

    fn time_to_position(&self, time_ns: u64) -> f64 {
        let imp = imp::SeekBar::from_obj(self);
        let ratio = time_ns as f64 / *imp.timeline_length.borrow() as f64;
        let width = imp.scale.width() as f64;
        width * ratio
    }

    pub fn get_scale(&self) -> Scale {
        let imp = imp::SeekBar::from_obj(self);
        imp.scale.get()
    }

    pub fn set_timeline_length(&self, timeline_length: u64) {
        let imp = imp::SeekBar::from_obj(self);
        *imp.timeline_length.borrow_mut() = timeline_length;
    }

    pub fn get_timeline_length(&self) -> u64 {
        let imp = imp::SeekBar::from_obj(self);
        *imp.timeline_length.borrow()
    }
}

