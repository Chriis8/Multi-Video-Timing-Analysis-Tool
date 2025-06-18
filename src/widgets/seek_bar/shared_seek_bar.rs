use gio::ListStore;
use glib::clone::Downgrade;
use gtk::prelude::*;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::{Box, TemplateChild, Button, FlowBox, ColumnView, SingleSelection, FlowBoxChild};
use crate::widgets::seek_bar::seek_bar::SeekBar;
use crate::widgets::video_player_widget::video_player::VideoPlayer;
use crate::widgets::split_panel::splits::VideoSegment;
use crate::widgets::split_panel::timeentry::TimeEntry;
use crate::widgets::split_panel::splittable::SplitTable;
use crate::widgets::sync::sync_manager::SyncManager;

use gstreamer::ClockTime;
use std::cell::{RefCell, Cell};
use std::rc::Rc;
use glib::WeakRef;
use std::time::Instant;
use crate::helpers::ui::flowbox_children;
use std::time::Duration;
use glib::timeout_add_local;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::helpers::data::borrow_asref_upgrade;


mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)] 
    #[template(resource = "/sharedseekbar/sharedseekbar.ui")]
    pub struct SharedSeekBar {
        #[template_child]
        pub seek_bar: TemplateChild<SeekBar>,
        #[template_child]
        pub previous_frame_button: TemplateChild<Button>,
        #[template_child]
        pub play_button: TemplateChild<Button>,
        #[template_child]
        pub next_frame_button: TemplateChild<Button>,
        #[template_child]
        pub jump_to_segment_button: TemplateChild<Button>,

        pub video_player_container: RefCell<Option<WeakRef<FlowBox>>>,
        pub split_table_column_view: RefCell<Option<WeakRef<ColumnView>>>,
        pub start_time_offset_liststore: RefCell<Option<WeakRef<ListStore>>>,
        pub split_table_liststore: RefCell<Option<WeakRef<ListStore>>>,
        pub sync_manager: RefCell<Option<WeakRef<SyncManager>>>,
        pub split_table: RefCell<Option<WeakRef<SplitTable>>>,

        pub is_dragging: Rc<Cell<bool>>,
        pub is_paused: Rc<Cell<bool>>,
        pub has_control: Rc<Cell<bool>>,
        pub scale_start_instant: Arc<Mutex<Option<Instant>>>,
        pub scale_start_offset: Rc<Cell<f64>>,
        pub starting_segment: Rc<Cell<u32>>,
        pub debounce_duration: RefCell<Duration>,
        pub last_click: Rc<RefCell<Option<Instant>>>,
    }

    #[gtk::glib::object_subclass]
    impl ObjectSubclass for SharedSeekBar {
        const NAME: &'static str = "SharedSeekBar";
        type Type = super::SharedSeekBar;
        type ParentType = Box;
        
        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }
        
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    
    impl SharedSeekBar {
        pub fn setup_buttons(&self) {
            println!("------------------setting up buttons");
            self.previous_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = seek_bar)] self.seek_bar,
                move |_| {
                    let video_player_container = borrow_asref_upgrade(&video_player_container_weak).ok().unwrap();

                    for child in flowbox_children(&video_player_container) {
                        let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                            Some(c) => c,
                            None => continue,
                        };

                        let content = match fb_child.child() {
                            Some(c) => c,
                            None => continue,
                        };

                        let video_player = match content.downcast_ref::<VideoPlayer>() {
                            Some(vp) => vp,
                            None => continue,
                        };

                        let scale = seek_bar.get_scale();
                        if scale.value() == 0.0 {
                            println!("scale is 0 skipping frame backward call");
                            return;
                        }

                        let arc = match video_player.pipeline().upgrade() {
                            Some(a) => a,
                            None => {
                                eprintln!("Shared jump to segment: Pipeline dropped");
                                continue
                            }
                        };

                        let pipeline = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        pipeline.frame_backward();
                    }
                }
            ));
            self.play_button.connect_clicked(glib::clone!(
                #[strong(rename_to = scale_start_instant)] self.scale_start_instant,
                #[strong(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = scale_start_offset)] self.scale_start_offset,
                #[strong(rename_to = is_paused)] self.is_paused,
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                #[strong(rename_to = last_click)] self.last_click,
                #[strong(rename_to = debounce_duration)] self.debounce_duration,
                move |_| {
                    let sync_manager = borrow_asref_upgrade(&sync_manager_weak).ok().unwrap();

                    let now = Instant::now();
                    let mut last_click = last_click.borrow_mut();

                    if let Some(last_time) = *last_click {
                        if now.duration_since(last_time) < *debounce_duration.borrow() {
                            println!("debouncing shared seek bar play button");
                            return;
                        }
                    }

                    *last_click = Some(now);
                    drop(last_click);
                    
                    let state = is_paused.get();
                    
                    if state == true {
                        sync_manager.clear_state();
                        sync_manager.set_on_all_playing(glib::clone!(
                            #[weak(rename_to = start_instant)] scale_start_instant,
                            move || {
                                let now = Instant::now();
                                println!("callbacked now: {now:?}");
                                *start_instant.lock().unwrap() = Some(now);
                            }
                        ));
                        sync_manager.play_videos();
                        scale_start_offset.set(seek_bar.get_scale().value());
                    } else {
                        sync_manager.pause_videos();
                    }
                    is_paused.set(!is_paused.get());
                }
            ));
            self.next_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = seek_bar)] self.seek_bar,
                move |_| {
                    let video_player_container = borrow_asref_upgrade(&video_player_container_weak).ok().unwrap();
                    
                    for child in flowbox_children(&video_player_container) {
                        let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                            Some(c) => c,
                            None => continue,
                        };

                        let content = match fb_child.child() {
                            Some(c) => c,
                            None => continue,
                        };

                        let video_player = match content.downcast_ref::<VideoPlayer>() {
                            Some(vp) => vp,
                            None => continue,
                        };

                        let scale = seek_bar.get_scale();
                        if scale.value() == 100.0 {
                            println!("scale is 100 skipping frame forward call");
                            return;
                        }

                        let arc = match video_player.pipeline().upgrade() {
                            Some(a) => a,
                            None => {
                                eprintln!("Shared jump to segment: Pipeline dropped");
                                continue
                            }
                        };

                        let pipeline = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };
                        pipeline.frame_forward();
                    }
                }
            ));
            self.jump_to_segment_button.connect_clicked(glib::clone!(
                #[strong(rename_to = split_table_column_view_weak)] self.split_table_column_view,
                #[strong(rename_to = starting_segment)] self.starting_segment,
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
                #[strong(rename_to = split_table_liststore_weak)] self.split_table_liststore,
                move |_| {
                    let split_table_column_view = borrow_asref_upgrade(&split_table_column_view_weak).ok().unwrap();
                    let video_player_container = borrow_asref_upgrade(&video_player_container_weak).ok().unwrap();
                    let split_table_liststore = borrow_asref_upgrade(&split_table_liststore_weak).ok().unwrap();
                    let split_table = borrow_asref_upgrade(&split_table_weak).ok().unwrap();
                    
                    match split_table_column_view.model().and_downcast::<SingleSelection>() {
                        Some(selection_model) => {
                            let selected_index = selection_model.selected();
                            starting_segment.set(selected_index);
                        },
                        None => {
                            starting_segment.set(0u32);
                        }
                    }

                    for child in flowbox_children(&video_player_container) {
                        let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                            Some(c) => c,
                            None => continue,
                        };

                        let content = match fb_child.child() {
                            Some(c) => c,
                            None => continue,
                        };

                        let video_player = match content.downcast_ref::<VideoPlayer>() {
                            Some(vp) => vp,
                            None => continue,
                        };

                        
                        let video_player_id = video_player.get_id();
                        let start_time_offset = split_table.get_offset_time_entry(video_player_id.as_str()).get_time();
                        let start_time_for_syncing = if starting_segment.get() == 0 { start_time_offset } else { 
                            split_table_liststore.item(starting_segment
                                .get()
                                .saturating_sub(1))
                                .and_downcast::<VideoSegment>()
                                .unwrap()
                                .get_time(video_player_id.as_str()
                        )};

                        let arc = match video_player.pipeline().upgrade() {
                            Some(a) => a,
                            None => {
                                eprintln!("Shared jump to segment: Pipeline dropped");
                                continue
                            }
                        };

                        let pipeline = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        println!("JUMP TO SEGMENT {video_player_id}: start_time_offset: {start_time_offset}");
                        println!("JUMP TO SEGMENT {video_player_id}: start_time_for_syncing (starting_segment_time): {start_time_for_syncing}");
                        if let Err(e) = pipeline.seek_position(ClockTime::from_nseconds(start_time_for_syncing)) {
                            eprintln!("Player {video_player_id} error setting position: {e}");
                        }
                    }
                }
            ));
        }

        pub fn setup_seek_bar_control(&self) {
            let shared_scale = self.seek_bar.get_scale();
            let _ = timeout_add_local(Duration:: from_millis(100), glib::clone!(
                #[strong(rename_to = is_dragging)] self.is_dragging,
                #[strong(rename_to = is_paused)] self.is_paused,
                #[strong(rename_to = has_control)] self.has_control,
                #[strong(rename_to = shared_scale)] shared_scale,
                #[strong(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = start_instant)] self.scale_start_instant,
                #[strong(rename_to = start_offset)] self.scale_start_offset,
                move || {
                    if is_dragging.get() || !has_control.get() || is_paused.get() {
                        let drag = is_dragging.get();
                        let control = has_control.get();
                        let paused = is_paused.get();

                        //println!("skipping flags: dragging: {drag}, control: {control}, paused: {paused}");
                        return glib::ControlFlow::Continue;
                    }
                    let instant = match *start_instant.lock().unwrap() {
                        Some(time) => time,
                        None => {
                            eprintln!("Error");
                            return glib::ControlFlow::Continue;
                        }
                    };

                    let current_time = Instant::now();
                    let scale_position = current_time.duration_since(instant);
                    let scale_position_ns = scale_position.as_nanos();
                    let timeline_length = seek_bar.get_timeline_length();
                    let scale_position_percent = (scale_position_ns as f64 / timeline_length as f64) * 100.0;

                    shared_scale.set_value(scale_position_percent + start_offset.get());
                    glib::ControlFlow::Continue
                }
            ));

            let gesture = gtk::GestureClick::new();
            gesture.connect_pressed(glib::clone!(
                #[weak(rename_to = is_dragging)] self.is_dragging,
                #[weak(rename_to = is_paused)] self.is_paused,
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                move |_,_,_x,_y| {
                    let video_player_container = borrow_asref_upgrade(&video_player_container_weak).ok().unwrap();
                    
                    for child in flowbox_children(&video_player_container) {
                        let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                            Some(c) => c,
                            None => continue,
                        };

                        let content = match fb_child.child() {
                            Some(c) => c,
                            None => continue,
                        };

                        let video_player = match content.downcast_ref::<VideoPlayer>() {
                            Some(vp) => vp,
                            None => continue,
                        };

                        let arc = match video_player.pipeline().upgrade() {
                            Some(a) => a,
                            None => {
                                eprintln!("Shared jump to segment: Pipeline dropped");
                                continue
                            }
                        };

                        let pipeline = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        pipeline.pause_video();
                    }
                    is_paused.set(true);
                    is_dragging.set(true);
                }
            ));

            gesture.connect_released(glib::clone!(
                #[weak(rename_to = is_dragging)] self.is_dragging,
                #[weak(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = start_time_offset_liststore_weak)] self.start_time_offset_liststore,
                move |_,_,_x,_y| {
                    let video_player_container = borrow_asref_upgrade(&video_player_container_weak).ok().unwrap();
                    let start_time_offset_liststore = borrow_asref_upgrade(&start_time_offset_liststore_weak).ok().unwrap();
                    
                    for (video_player_index, child) in flowbox_children(&video_player_container).enumerate() {
                        let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                            Some(c) => c,
                            None => continue,
                        };

                        let content = match fb_child.child() {
                            Some(c) => c,
                            None => continue,
                        };

                        let video_player = match content.downcast_ref::<VideoPlayer>() {
                            Some(vp) => vp,
                            None => continue,
                        };

                        let arc = match video_player.pipeline().upgrade() {
                            Some(a) => a,
                            None => {
                                eprintln!("Shared jump to segment: Pipeline dropped");
                                continue
                            }
                        };

                        let pipeline = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        let offset_time_entry = start_time_offset_liststore.item(video_player_index as u32).and_downcast::<TimeEntry>().unwrap();
                        let offset_time = offset_time_entry.get_time();
                        let percent_position = seek_bar.get_scale().value() / 100.0;
                        let position = (percent_position * seek_bar.get_timeline_length() as f64) as u64;
                        let clock_time_position = ClockTime::from_nseconds(position + offset_time);
                        pipeline.seek_position(clock_time_position).expect("Failed to seek to synced position");
                    }
                    is_dragging.set(false);
                }
                
            ));
            gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
            self.seek_bar.add_controller(gesture);
        }

    }
    
    impl ObjectImpl for SharedSeekBar {
        fn constructed(&self) {
        }

        fn dispose(&self) {
        
        }
    }
    impl WidgetImpl for SharedSeekBar {}
    impl BoxImpl for SharedSeekBar {}
}

glib::wrapper! {
    pub struct SharedSeekBar(ObjectSubclass<imp::SharedSeekBar>)
    @extends gtk::Widget,
    @implements gtk::Buildable;
}

impl SharedSeekBar {
    pub fn new(video_player_container: &FlowBox, split_table_column_view: &ColumnView, start_time_offset_liststore: &ListStore, split_table_liststore: &ListStore, sync_manager: &SyncManager, split_table: &SplitTable) -> Self {
        let widget: Self = glib::Object::new::<Self>();
        let imp = imp::SharedSeekBar::from_obj(&widget);
        imp.seek_bar.set_auto_timeline_length_handling(true);
        imp.seek_bar.update_marks_on_width_change_timeout();
        imp.video_player_container.borrow_mut().replace(Downgrade::downgrade(video_player_container));
        imp.split_table_column_view.borrow_mut().replace(Downgrade::downgrade(split_table_column_view));
        imp.start_time_offset_liststore.borrow_mut().replace(Downgrade::downgrade(start_time_offset_liststore));
        imp.split_table_liststore.borrow_mut().replace(Downgrade::downgrade(split_table_liststore));
        imp.sync_manager.borrow_mut().replace(Downgrade::downgrade(sync_manager));
        imp.split_table.borrow_mut().replace(Downgrade::downgrade(split_table));
        *imp.scale_start_instant.lock().unwrap() = None;
        *imp.debounce_duration.borrow_mut() = Duration::from_millis(200);
        imp.setup_buttons();
        imp.setup_seek_bar_control();
        widget.set_controls(false);
        widget
    }

    pub fn connect_row(&self, row_index: u32) {
        let imp = self.imp();
        let split_table = borrow_asref_upgrade(&imp.split_table).ok().unwrap();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();
        let video_player_container = borrow_asref_upgrade(&imp.video_player_container).ok().unwrap();
                
        let row = split_table_liststore.item(row_index).and_downcast::<VideoSegment>().unwrap();
        let row_count = split_table_liststore.n_items();
        //let colors = vec!["red", "blue", "green", "black", "coral", "lavender"];
        for child in flowbox_children(&video_player_container) {
            let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                Some(c) => c,
                None => continue,
            };

            let content = match fb_child.child() {
                Some(c) => c,
                None => continue,
            };

            let video_player = match content.downcast_ref::<VideoPlayer>() {
                Some(vp) => vp,
                None => continue,
            };

            let video_player_id = video_player.get_id().to_string();
            let color = video_player.get_color();
            let time = row.get_time_entry_copy(video_player_id.as_str());
            let offset_time_entry = split_table.get_offset_time_entry(video_player_id.as_str());

            // id should always be row_count regardless of if the row is inserted in the middle.
            // not sure if it will matter but this should give marks unique ids
            let segment_id = row.get_segment_id(); 
            imp.seek_bar.add_mark(format!("video-{video_player_id}, segment-{segment_id}"), time, color.as_str(), offset_time_entry);
        }
    }

    pub fn connect_column(&self, video_player_id: &str, color: &str) {
        let imp = self.imp();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();
        let split_table = borrow_asref_upgrade(&imp.split_table).ok().unwrap();
        
        let row_count = split_table_liststore.n_items();
        for i in 0..row_count {
            let row = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let time = row.get_time_entry_copy(video_player_id);
            let offset = split_table.get_offset_time_entry(video_player_id);
            let segment_id = row.get_segment_id();
            // id are given in order as they have already been created
            imp.seek_bar.add_mark(format!("video-{video_player_id}, segment-{segment_id}"), time, color, offset);
        }
    }

    pub fn update_timeline_length(&self) {
        let imp = self.imp();
        imp.seek_bar.update_timeline_length();
    }

    pub fn toggle_has_control(&self) {
        let imp = self.imp();
        let video_player_container = borrow_asref_upgrade(&imp.video_player_container).ok().unwrap();
        let split_table = borrow_asref_upgrade(&imp.split_table).ok().unwrap();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();
        let sync_manager = borrow_asref_upgrade(&imp.sync_manager).ok().unwrap();
        
        let status = imp.has_control.get();
        for child in flowbox_children(&video_player_container) {
            let fb_child = match child.downcast_ref::<FlowBoxChild>() {
                Some(c) => c,
                None => continue,
            };

            let content = match fb_child.child() {
                Some(c) => c,
                None => continue,
            };

            let video_player = match content.downcast_ref::<VideoPlayer>() {
                Some(vp) => vp,
                None => continue,
            };

            let arc = match video_player.pipeline().upgrade() {
                Some(a) => a,
                None => {
                    eprintln!("Shared jump to segment: Pipeline dropped");
                    continue
                }
            };

            let pipeline = match arc.lock() {
                Ok(g) => g,
                Err(_) => {
                    eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                    continue
                }
            };
            pipeline.pause_video();
            let video_player_id = video_player.get_id().to_string();
            if !status {
                let offset = split_table.get_offset_time_entry(video_player_id.as_str());
                let start_time = gstreamer::ClockTime::from_nseconds(offset.get_time());
                let mut end_time = pipeline.get_length().unwrap();
                if split_table_liststore.n_items() > 0 {
                    match split_table.get_previous_time(video_player_id.as_str(), split_table_liststore.n_items() - 1) {
                        Some(time) => { end_time = time },
                        None => {
                            let pipeline_length = pipeline.get_length().unwrap();
                            if pipeline_length > imp.seek_bar.get_timeline_length() {
                                imp.seek_bar.set_timeline_length(pipeline_length);
                            }
                            end_time = pipeline_length;
                        }
                    }
                } else {
                    let pipeline_length = pipeline.get_length().unwrap();
                    if pipeline_length > imp.seek_bar.get_timeline_length() {
                        imp.seek_bar.set_timeline_length(pipeline_length);
                    }
                }
                pipeline.set_start_clamp(start_time.nseconds());
                pipeline.set_end_clamp(end_time);
                if let Err(e) = pipeline.seek_position(start_time) {
                    eprintln!("Player {video_player_id} error setting position: {e}");
                }
                imp.seek_bar.get_scale().set_value(0.0);
                drop(pipeline);
                if let Err(e) = sync_manager.sync_clocks() {
                    eprintln!("Error syncing clocks {e}");
                }
            } else {
                pipeline.reset_clamps();
                drop(pipeline);
                if let Err(e) = sync_manager.unsync_clocks() {
                    eprintln!("Error unsyncing clocks {e}");  
                }
            }
            video_player.set_controls(status);
        }
        imp.is_paused.set(true);
        imp.has_control.set(!status);
        self.set_controls(!status);
    }

    pub fn get_control_state(&self) -> bool {
        let imp = self.imp();
        return imp.has_control.get();
    }

    fn set_controls(&self, status: bool) {
        let imp = self.imp();
        if status {
            imp.seek_bar.remove_css_class("scale-slider-hidden");
        } else {
            imp.seek_bar.add_css_class("scale-slider-hidden");
        }
        imp.seek_bar.set_sensitive(status);
        imp.jump_to_segment_button.set_sensitive(status);
        imp.next_frame_button.set_sensitive(status);
        imp.play_button.set_sensitive(status);
        imp.previous_frame_button.set_sensitive(status);
    }

    pub fn remove_marks(&self, video_player_id: &str) {
        let imp = self.imp();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();

        for i in 0..split_table_liststore.n_items() {
            let segment = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let segment_id = segment.get_segment_id();
            imp.seek_bar.remove_mark(&format!("video-{video_player_id}, segment-{segment_id}"));
        }

        self.update_timeline_length();
    }
}

