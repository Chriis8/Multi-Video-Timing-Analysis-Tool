use glib::random_int;
use gtk::prelude::*;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::{Label, Overlay, Box, Scale, TemplateChild, Fixed};
use std::collections::HashSet;
use std::{cell::{RefCell, Cell}, collections::HashMap};
use std::rc::Rc;
use crate::widgets::split_panel::timeentry::TimeEntry;
use glib::{SourceId, source};
use gtk::prelude::WidgetExtManual;

mod imp {

    use super::*;
    
    #[derive(CompositeTemplate, Default)] 
    #[template(resource = "/seekbar/seekbar.ui")]
    pub struct SeekBar {
        pub marks: RefCell<HashMap<String, (Rc<TimeEntry>, gtk::Widget)>>,
        pub timeline_length: Rc<RefCell<u64>>,
        pub timeline_dirty_flag: RefCell<bool>,
        pub auto_length_from_marks: RefCell<bool>,
        pub updating_scale_width_timeout: RefCell<bool>,
        pub last_width: Cell<i32>,

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
    pub fn new(timeline_length: u64, auto_timeline_length_handling: bool) -> Self {
        let widget: Self = glib::Object::new::<Self>();
        let imp = imp::SeekBar::from_obj(&widget);
        *imp.timeline_length.borrow_mut() = timeline_length;
        *imp.auto_length_from_marks.borrow_mut() = auto_timeline_length_handling;
        *imp.updating_scale_width_timeout.borrow_mut() = false;
        widget
    }

    pub fn add_tick_callback_timeout(&self) {
        let self_weak = self.downgrade();
        let source_id = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            if let Some(this) = self_weak.upgrade(){
                let imp = this.imp();
                let current_width = imp.scale.allocation().width();
                if imp.last_width.get() != current_width {
                    imp.last_width.set(current_width);
                    this.update_mark_positions();
                }
            }
            glib::ControlFlow::Continue
        });
    }


    pub fn add_mark(&self, id: String, time_entry: Rc<TimeEntry>, color: &str) {
        let imp = imp::SeekBar::from_obj(self);
        
        let mark = Label::new(None);
        mark.set_markup(&format!("<span foreground='{color}'>^</span>"));
        mark.set_halign(gtk::Align::Center);
        mark.set_valign(gtk::Align::Center);
        mark.set_visible(false);

        imp.fixed.put(&mark, 0.0, 0.0);
        //self.update_mark_position(&mark.clone().upcast(), &time_entry);

        let entry_time = time_entry.get_time();
        if entry_time != u64::MAX && entry_time > *imp.timeline_length.borrow() && *imp.auto_length_from_marks.borrow() {
            *imp.timeline_dirty_flag.borrow_mut() = true;
        }

        time_entry.connect_notify_local(Some("time"), glib::clone!(
            #[strong(rename_to = fixed_overlay)] imp.fixed.clone(),
            #[strong(rename_to = mark_clone)] mark.clone(),
            #[strong(rename_to = timeline_length)] imp.timeline_length,
            #[strong(rename_to = scale)] imp.scale,
            #[strong(rename_to = dirty_flag)] imp.timeline_dirty_flag,
            #[weak(rename_to = this)] self,
            #[strong(rename_to = auto_length)] imp.auto_length_from_marks,
            move |time_entry, _| {
                let old_time = if time_entry.get_old_time() == u64::MAX { 0 } else { time_entry.get_old_time()};
                let time = time_entry.get_time();
                let width = scale.width();
                let widget_width = mark_clone.allocated_width();
                
                if (old_time == *timeline_length.borrow() || time > *timeline_length.borrow()) && *auto_length.borrow() {
                    *dirty_flag.borrow_mut() = true;
                }
                if *dirty_flag.borrow() {
                    this.update_timeline_length();
                }

                if *timeline_length.borrow() == 0 {
                    eprintln!("Timeline_length is still 0");
                    return;
                }
                if time == u64::MAX {
                    mark_clone.set_visible(false);
                    let percent = 0.0;
                    let x = percent * (width - 4) as f64;
                    fixed_overlay.move_(&mark_clone, x - (widget_width as f64 / 2.0) + 2.0, 25.0);
                } else {
                    mark_clone.set_visible(true);
                    let percent = time as f64 / *timeline_length.borrow() as f64;
                    let x = percent * (width - 4) as f64;
                    fixed_overlay.move_(&mark_clone, x - (widget_width as f64 / 2.0) + 2.0, 25.0);
                }
            }
        ));
        
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

    fn update_mark_positions(&self) {
        let imp = imp::SeekBar::from_obj(self);
        for (time_entry, widget) in imp.marks.borrow().values() {
            let widget_width = widget.allocated_width();
            let time = time_entry.get_time();
            if *imp.timeline_length.borrow() == 0 {
                return;
            }
            if time == u64::MAX {
                widget.set_visible(false);
                let percent = 0.0;
                let x_pos = percent * (imp.scale.width() - 4) as f64;
                imp.fixed.move_(widget, x_pos - (widget_width as f64 / 2.0), 25.0);
            } else {
                widget.set_visible(true);
                let percent = time as f64 / *imp.timeline_length.borrow() as f64;
                let x_pos = percent * (imp.scale.width() - 4) as f64;
                imp.fixed.move_(widget, x_pos - (widget_width as f64 / 2.0), 25.0);
            }
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
        self.update_mark_positions();
    }

    pub fn get_timeline_length(&self) -> u64 {
        let imp = imp::SeekBar::from_obj(self);
        *imp.timeline_length.borrow()
    }

    fn update_timeline_length(&self) {
        let imp = imp::SeekBar::from_obj(self);
        let largest_time = imp.marks
            .borrow()
            .values()
            .filter_map(|(time_entry, _)| {
                let time = time_entry.get_time();
                if time == u64::MAX {
                    Some(0)
                } else {
                    Some(time)
                }
            })
            .max()
            .unwrap_or(0);
        
        *imp.timeline_length.borrow_mut() = largest_time;
        *imp.timeline_dirty_flag.borrow_mut() = false;
        self.update_mark_positions();
    }

    pub fn set_auto_timeline_length_handling(&self, flag: bool) {
        let imp = imp::SeekBar::from_obj(self);
        *imp.auto_length_from_marks.borrow_mut() = flag;
    }
}

