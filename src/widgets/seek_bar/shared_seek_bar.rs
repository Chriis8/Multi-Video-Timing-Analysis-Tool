use gio::ListStore;
use glib::clone::Downgrade;
use gtk::prelude::*;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::{Box, TemplateChild, Button, FlowBox, ColumnView, SingleSelection, FlowBoxChild};
use crate::widgets::seek_bar::seek_bar::SeekBar;
use crate::widgets::sync::sync_manager::SyncEvent;
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
use gstreamer::prelude::ClockExt;


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
        pub starting_segment: Rc<Cell<u32>>,
        pub debounce_duration: RefCell<Duration>,
        pub last_click: Rc<RefCell<Option<Instant>>>,
        pub seek_bar_update_timeout: Rc<RefCell<Option<glib::SourceId>>>,
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
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                move |_| {
                    let sync_manager = borrow_asref_upgrade(&sync_manager_weak).ok().unwrap();
                    sync_manager.frame_backward();
                }
            ));
            self.play_button.connect_clicked(glib::clone!(
                #[strong(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = is_paused)] self.is_paused,
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                #[strong(rename_to = last_click)] self.last_click,
                #[strong(rename_to = debounce_duration)] self.debounce_duration,
                #[strong(rename_to = split_table_weak)] self.split_table,
                move |_| {
                    let sync_manager = borrow_asref_upgrade(&sync_manager_weak).ok().unwrap();
                    let split_table = borrow_asref_upgrade(&split_table_weak).ok().unwrap();

                    let now = Instant::now();
                    let mut last_click = last_click.borrow_mut();

                    //Debounce if time since last click is less than debounce duration
                    if let Some(last_time) = *last_click {
                        if now.duration_since(last_time) < *debounce_duration.borrow() {
                            println!("debouncing shared seek bar play button");
                            return;
                        }
                    }

                    //update last click time
                    *last_click = Some(now);
                    drop(last_click);
                    
                    let state = is_paused.get();
                    
                    //toggle being playing and pausing all videos
                    if state == true {
                        //Record the start time offset for each video
                        let mut offsets: HashMap<String, u64> = HashMap::new();
                        let offsets_row_map = split_table.get_start_time_offset_row_map();
                        for (video_player_id, offset_time_entry) in offsets_row_map.borrow().iter() {
                            offsets.insert(video_player_id.to_string(), offset_time_entry.get_time());
                        }
                        
                        sync_manager.play_videos(offsets);
                    } else {
                        sync_manager.pause_videos();
                    }
                    //update is_paused flag
                    is_paused.set(!is_paused.get());
                }
            ));
            self.next_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                move |_| {
                    let sync_manager = borrow_asref_upgrade(&sync_manager_weak).ok().unwrap();
                    sync_manager.frame_forward();
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

            //Pauses videos when seek bar is pressed
            let gesture = gtk::GestureClick::new();
            gesture.connect_pressed(glib::clone!(
                #[weak(rename_to = is_dragging)] self.is_dragging,
                #[weak(rename_to = is_paused)] self.is_paused,
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                move |_,_,_x,_y| {
                    let sync_manager = borrow_asref_upgrade(&sync_manager_weak).ok().unwrap();
                    if !is_paused.get() {
                        sync_manager.pause_videos();
                    }
                    is_paused.set(true);
                    is_dragging.set(true);
                }
            ));

            //Seeks to position based on seek bar value when released
            gesture.connect_released(glib::clone!(
                #[weak(rename_to = is_dragging)] self.is_dragging,
                #[weak(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                #[strong(rename_to = split_table_weak)] self.split_table,
                move |_,_,_x,_y| {
                    let sync_manager = borrow_asref_upgrade(&sync_manager_weak).ok().unwrap();
                    let split_table = borrow_asref_upgrade(&split_table_weak).ok().unwrap();

                    //Record the absolute positions for each video to use to perform the seek
                    let mut clock_positions: HashMap<String, ClockTime> = HashMap::new();
                    let start_time_offset_row_map = split_table.get_start_time_offset_row_map();
                    for (video_player_id, offset) in start_time_offset_row_map.borrow().iter() {
                        let offset_time = offset.get_time();
                        let percent_position = seek_bar.get_scale().value() / 100.0;
                        let position = (percent_position * seek_bar.get_timeline_length() as f64) as u64;
                        
                        //Relative position + start time offset
                        let clock_time_position = ClockTime::from_nseconds(position + offset_time);
                        clock_positions.insert(video_player_id.to_string(), clock_time_position);
                    }
                    
                    //Perform seek operation on all video passing in the absolute positions
                    sync_manager.seek(clock_positions);
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
        *imp.debounce_duration.borrow_mut() = Duration::from_millis(200);
        imp.setup_buttons();
        imp.setup_seek_bar_control();
        widget.set_controls(false);
        widget
    }

    //Connect split table row to seek bar mark manager
    //Input: split table row index to add
    pub fn connect_row(&self, row_index: u32) {
        let imp = self.imp();
        let split_table = borrow_asref_upgrade(&imp.split_table).ok().unwrap();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();
        let video_player_container = borrow_asref_upgrade(&imp.video_player_container).ok().unwrap();

        let row = split_table_liststore.item(row_index).and_downcast::<VideoSegment>().unwrap();
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
            
            //Retrieve color, time, and start time offset for the given video as identifying information for any given mark
            let color = video_player.get_color();
            let time = row.get_time_entry_copy(video_player_id.as_str());
            let offset_time_entry = split_table.get_offset_time_entry(video_player_id.as_str());

            let segment_id = row.get_segment_id();

            imp.seek_bar.add_mark(format!("video-{video_player_id}, segment-{segment_id}"), time, color.as_str(), offset_time_entry);
        }
    }

    //Connect split table column to seek bar mark manager
    //Inputs: video_player_id: Id of new video player marks being added, Color: color assigned to the video player
    pub fn connect_column(&self, video_player_id: &str, color: &str) {
        let imp = self.imp();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();
        let split_table = borrow_asref_upgrade(&imp.split_table).ok().unwrap();
        
        let row_count = split_table_liststore.n_items();
        for i in 0..row_count {
            //Retrieve time and start time offset for each given row associate with the video player
            let row = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let time = row.get_time_entry_copy(video_player_id);
            let offset = split_table.get_offset_time_entry(video_player_id);
            let segment_id = row.get_segment_id();

            imp.seek_bar.add_mark(format!("video-{video_player_id}, segment-{segment_id}"), time, color, offset);
        }
    }

    //Updates the seek bar max value
    //i.e. Scale: 0.0 - 100.0 -> 0 ns - Video Duration in ns
    pub fn update_timeline_length(&self) {
        let imp = self.imp();
        imp.seek_bar.update_timeline_length();
    }

    //Toggles between individual and synced video control
    pub fn toggle_has_control(&self) {
        let imp = self.imp();
        if imp.has_control.get() {
            self.release_shared_control();
        } else {
            self.take_shared_control();
        }
    }

    //Get if video players are in individual or shared control
    pub fn get_control_state(&self) -> bool {
        let imp = self.imp();
        return imp.has_control.get();
    }

    //Disable or enables shared video player controls
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

    //Remove marks from shared seek bar
    //Inputs: video player id of the marks to be removed
    pub fn remove_marks(&self, video_player_id: &str) {
        let imp = self.imp();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();

        //Loops through segments to remove
        for i in 0..split_table_liststore.n_items() {
            let segment = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let segment_id = segment.get_segment_id();

            //Remove mark
            imp.seek_bar.remove_mark(&format!("video-{video_player_id}, segment-{segment_id}"));
        }

        //Updates length of seek bar to reflect new mark state
        self.update_timeline_length();
    }

    //Enables shared video control
    pub fn take_shared_control(&self) {
        let imp = self.imp();
        let video_player_container = borrow_asref_upgrade(&imp.video_player_container).ok().unwrap();
        let split_table = borrow_asref_upgrade(&imp.split_table).ok().unwrap();
        let split_table_liststore = borrow_asref_upgrade(&imp.split_table_liststore).ok().unwrap();
        let sync_manager = borrow_asref_upgrade(&imp.sync_manager).ok().unwrap();
        
        //Loops through each video player
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

            let mut pipeline = match arc.lock() {
                Ok(g) => g,
                Err(_) => {
                    eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                    continue
                }
            };
            pipeline.pause_video();

            //Find start and end times to clamp video
            //Start: start time offset
            //End: Position of last mark otherwise end of file
            let video_player_id = video_player.get_id().to_string();
            let offset = split_table.get_offset_time_entry(video_player_id.as_str());
            let start_time = gstreamer::ClockTime::from_nseconds(offset.get_time());
            let mut end_time = pipeline.get_length().unwrap();
            if split_table_liststore.n_items() > 0 {
                match split_table.get_previous_time(video_player_id.as_str(), split_table_liststore.n_items()) {
                    Some(time) => { end_time = time },
                    None => {
                        if end_time > imp.seek_bar.get_timeline_length() {
                            //Seek bar only updates length based on mark positions. If the end time of a video without marks exceeds this we update the length.
                            imp.seek_bar.set_timeline_length(end_time);
                        }
                    }
                }
            } else {
                if end_time > imp.seek_bar.get_timeline_length() {
                    //Seek bar only updates length based on mark positions. If the end time of a video without marks exceeds this we update the length.
                    imp.seek_bar.set_timeline_length(end_time);
                }
            }

            //Applies the clamp to the pipeline
            let _ = pipeline.apply_clamp(start_time, ClockTime::from_nseconds(end_time));

            //Seeks to the start time offset
            if let Err(e) = pipeline.seek_position(start_time) {
                eprintln!("Player {video_player_id} error setting position: {e}");
            }

            //Disable individual user control while in shared control
            video_player.set_controls(false);
        }

        //Update seek bar value to match starting position of the videos
        imp.seek_bar.get_scale().set_value(0.0);

        imp.is_paused.set(true);
        imp.has_control.set(true);
        self.set_controls(true);
    }

    //Disable shared video control
    pub fn release_shared_control(&self) {
        let imp = self.imp();
        let video_player_container = borrow_asref_upgrade(&imp.video_player_container).ok().unwrap();
        let sync_manager = borrow_asref_upgrade(&imp.sync_manager).ok().unwrap();
        
        //Loops through each of the video players
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

            let mut pipeline = match arc.lock() {
                Ok(g) => g,
                Err(_) => {
                    eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                    continue
                }
            };
            //Pause and reset clamps
            pipeline.pause_video();
            let _ = pipeline.reset_clamps();
            drop(pipeline);

            //Enables individual user controls for video player
            video_player.set_controls(true);
        }

        //Unsync the pipelines clocks
        if let Err(e) = sync_manager.unsync_clocks() {
            eprintln!("Error unsyncing clocks {e}");  
        }

        imp.is_paused.set(true);
        imp.has_control.set(false);
        self.set_controls(false);
    }

    //Start updating scale position while playing synced videos
    //Inputs: base time videos are using, position of the scale at the time of playing
    pub fn start_progress(&self, base_time: ClockTime, scale_position: ClockTime) {
        let imp = self.imp();
        
        let seek_bar = imp.seek_bar.clone();
        let sync_manager = borrow_asref_upgrade(&imp.sync_manager).ok().unwrap();
        let clock = sync_manager.get_shared_clock();
        let timeline_length = ClockTime::from_nseconds(imp.seek_bar.get_timeline_length());
        let timeout_ref = imp.seek_bar_update_timeout.clone();

        //Update scale position every 200 milliseconds
        *imp.seek_bar_update_timeout.borrow_mut() = Some(glib::timeout_add_local(
            Duration::from_millis(200),
            move || {
                if let Some(current_time) = clock.time() {
                    if current_time >= base_time {
                        //Calculate absolute scale position
                        let media_time = current_time - base_time + scale_position;
                        let position = media_time.nseconds() as f64;
                        let new_scale_position = (position / seek_bar.get_timeline_length() as f64) * 100.0;
                        
                        //Set scale position
                        seek_bar.get_scale().set_value(new_scale_position);

                        //Stop updating when video reaches the end
                        if media_time >= timeline_length {
                            *timeout_ref.borrow_mut() = None;
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                glib::ControlFlow::Continue
            }
        ));
    }

    //Stops updating the scale position
    pub fn stop_progress(&self) {
        let imp = self.imp();
        if let Some(timeout_id) = imp.seek_bar_update_timeout.take() {
            timeout_id.remove();
        }
    }

    //Handles events sent from the sync manager
    pub fn handle_sync_event(&self, event: SyncEvent) {
        match event {
            SyncEvent::SyncEnabled { base_time } => {
                println!("SyncEnabled: Videos synced base_time {base_time}");
            },
            SyncEvent::SyncDisabled => {
                println!("SyncDisabled: Video unsynced");
            },
            SyncEvent::PlaybackStarted { base_time, scale_position} => {
                //Start scale position updating
                println!("PlaybackStarted: base_time = {base_time}, scale_position = {scale_position}");
                self.start_progress(base_time, scale_position);
            },
            SyncEvent::PlaybackPaused => {
                //Stop scale position updating
                println!("PlaybackPaused");
                self.stop_progress();
            },
            SyncEvent::Seeked => {
                println!("Seeked");
            },
        }
    }
}

