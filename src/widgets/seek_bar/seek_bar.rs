use gtk::prelude::*;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::{Label, Overlay, Box, Scale, TemplateChild, Fixed};
use std::{cell::{RefCell, Cell}, collections::HashMap};
use std::rc::Rc;
use crate::widgets::split_panel::timeentry::TimeEntry;

mod imp {

    use super::*;
    
    #[derive(CompositeTemplate, Default)] 
    #[template(resource = "/seekbar/seekbar.ui")]
    pub struct SeekBar {
        pub marks: RefCell<HashMap<String, (TimeEntry, TimeEntry, gtk::Widget)>>,
        pub timeline_length: Rc<RefCell<u64>>,
        pub timeline_dirty_flag: RefCell<bool>,
        pub auto_length_from_marks: RefCell<bool>,
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
        widget
    }

    //Updates the marks when the width of the scale widget changes
    pub fn update_marks_on_width_change_timeout(&self) {
        let self_weak = self.downgrade();
        let source_id = glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            if let Some(this) = self_weak.upgrade(){
                let imp = this.imp();
                
                //update current width if it changes since last check
                let current_width = imp.scale.allocation().width();
                if imp.last_width.get() != current_width {
                    imp.last_width.set(current_width);
                    //updates mark positions based on new width
                    this.update_mark_positions();
                }
            }
            glib::ControlFlow::Continue
        });
    }
    
    //Add new mark to seek bar
    //Inputs: Id - (video_player_id and segment_id), TimeEntry - split table cell entre, color - video player's associated mark color, offset - start time offset for the video
    pub fn add_mark(&self, id: String, time_entry: TimeEntry, color: &str, offset: TimeEntry) {
        let imp = imp::SeekBar::from_obj(self);
        
        //Setup mark widget
        let mark = Label::new(None);
        mark.set_markup(&format!("<span foreground='{color}'>^</span>"));
        mark.set_halign(gtk::Align::Center);
        mark.set_valign(gtk::Align::Center);
        mark.set_visible(false);

        //Set default position while not visible
        imp.fixed.put(&mark, 0.0, 0.0);


        let entry_time = if time_entry.get_time() == u64::MAX { u64::MAX } else { time_entry.get_time() - offset.get_time()};

        //Update timeline length dirty flag if new mark would affect timeline length
        if entry_time != u64::MAX && entry_time > *imp.timeline_length.borrow() && *imp.auto_length_from_marks.borrow() {
            *imp.timeline_dirty_flag.borrow_mut() = true;
        }

        //Updates mark position if associated time entry changes
        time_entry.connect_notify_local(Some("time"), glib::clone!(
            #[strong(rename_to = fixed_overlay)] imp.fixed.clone(),
            #[strong(rename_to = mark_clone)] mark.clone(),
            #[strong(rename_to = timeline_length)] imp.timeline_length,
            #[strong(rename_to = scale)] imp.scale,
            #[strong(rename_to = dirty_flag)] imp.timeline_dirty_flag,
            #[weak(rename_to = this)] self,
            #[strong(rename_to = auto_length)] imp.auto_length_from_marks,
            #[strong(rename_to = offset)] offset,
            move |time_entry, _| {
                //Retrieve new and old time position
                let old_time = if time_entry.get_old_time() == u64::MAX { 0 } else { time_entry.get_old_time() - offset.get_time()};
                let time = time_entry.get_time().checked_sub(offset.get_time());
                let width = scale.width();
                
                //Update timeline length dirt flag if the new time would affect timeline length and auto timeline length handling is enabled
                if (old_time == *timeline_length.borrow() || time.unwrap() > *timeline_length.borrow()) && *auto_length.borrow() {
                    *dirty_flag.borrow_mut() = true;
                }
                //Update timeline length if dirty flag is set
                if *dirty_flag.borrow() {
                    this.update_timeline_length();
                }
                
                if *timeline_length.borrow() == 0 {
                    eprintln!("Timeline_length is still 0");
                    return;
                }
                //Removes mark from view if time entry is unset
                if time_entry.get_time() == u64::MAX || time.is_none() {
                    mark_clone.set_visible(false);
                    fixed_overlay.move_(&mark_clone, 0.0, 25.0);
                } else {
                    //Displays mark
                    mark_clone.set_visible(true);

                    //Finds mark position based on time entries time and adjusts precise position based on the marks widget size.
                    let (_min, natural, _min_b, _nat_b) = mark_clone.measure(gtk::Orientation::Horizontal, -1);
                    let widget_width = natural;
                    let percent = time.unwrap() as f64 / *timeline_length.borrow() as f64;
                    let x = percent * (width - 4) as f64;
                    
                    //Sets the mark position
                    fixed_overlay.move_(&mark_clone, x - (widget_width as f64 / 2.0) + 2.0, 25.0);
                }
            }
        ));
        
        //Inserts new mark and additional information into map
        imp.marks.borrow_mut().insert(id.clone(), (time_entry, offset, mark.clone().upcast()));
    }

    //Removes mark for seek bar
    pub fn remove_mark(&self, id: &str) {
        let imp = imp::SeekBar::from_obj(self);
        //Removes mark from hash map and removes widget from display
        if let Some((_, _, widget)) = imp.marks.borrow_mut().remove(id) {
            widget.unparent();
        }
    }

    //Updates mark position - called usually if the length of the timeline changes
    pub fn update_mark_positions(&self) {
        let imp = imp::SeekBar::from_obj(self);
        //Loops through each mark
        for (time_entry, offset, widget) in imp.marks.borrow().values() {
            let time = time_entry.get_time().checked_sub(offset.get_time());
            if *imp.timeline_length.borrow() == 0 {
                return;
            }
            //Hides mark if time entry unset
            if time_entry.get_time() == u64::MAX || time.is_none() {
                widget.set_visible(false);
                imp.fixed.move_(widget, 0.0, 25.0);
            } else {
                //Displays mark and finds precise positioning 
                widget.set_visible(true);
                let (_min, natural, _min_b, _nat_b) = widget.measure(gtk::Orientation::Horizontal, -1);
                    let widget_width = natural;
                let percent = time.unwrap() as f64 / *imp.timeline_length.borrow() as f64;
                let x_pos = percent * (imp.scale.width() - 4) as f64;

                //Sets the marks new position
                imp.fixed.move_(widget, x_pos - (widget_width as f64 / 2.0), 25.0);
            }
        }
    }

    //Gets the seek bar widget object
    pub fn get_scale(&self) -> Scale {
        let imp = imp::SeekBar::from_obj(self);
        imp.scale.get()
    }

    //Updates timeline length of the seek bar
    pub fn set_timeline_length(&self, timeline_length: u64) {
        let imp = imp::SeekBar::from_obj(self);
        *imp.timeline_length.borrow_mut() = timeline_length;

        //Update any currently store marks on the seek bar
        self.update_mark_positions();
    }

    //Gets the timeline length of the seek bar
    pub fn get_timeline_length(&self) -> u64 {
        let imp = imp::SeekBar::from_obj(self);
        *imp.timeline_length.borrow()
    }

    //Updates the timeline length of the seek bar based on the positions of the currently stored marks
    pub fn update_timeline_length(&self) {
        let imp = imp::SeekBar::from_obj(self);
        //Retrieves the mark that is furthest along on the seek bar
        let largest_time = imp.marks
            .borrow()
            .values()
            .filter_map(|(time_entry, offset, _)| {
                let time = time_entry.get_time();
                if time == u64::MAX || time.checked_sub(offset.get_time()).is_none() {
                    Some(0)
                } else {
                    Some(time.saturating_sub(offset.get_time()))
                }
            })
            .max()
            .unwrap_or(0);
        
        *imp.timeline_length.borrow_mut() = largest_time;
        *imp.timeline_dirty_flag.borrow_mut() = false;
        
        //Update any currently store marks on the seek bar
        self.update_mark_positions();
    }

    //Sets automatic timeline length handling whenever marks are added or change position 
    pub fn set_auto_timeline_length_handling(&self, flag: bool) {
        let imp = imp::SeekBar::from_obj(self);
        *imp.auto_length_from_marks.borrow_mut() = flag;
    }

    pub fn reset_all_marks(&self) {
        let imp = self.imp();
        for (_, (_, _, widget)) in imp.marks.borrow_mut().drain() {
            widget.unparent();
        }
        self.update_timeline_length();
    }
}

