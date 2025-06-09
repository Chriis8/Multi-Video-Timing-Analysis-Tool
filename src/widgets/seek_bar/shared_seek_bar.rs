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


mod imp {
    use super::*;
    
    #[derive(CompositeTemplate, Default)] 
    #[template(resource = "/sharedseekbar/sharedseekbar.ui")]
    pub struct SharedSeekBar {
        #[template_child]
        pub seek_bar: TemplateChild<SeekBar>,
        #[template_child]
        pub previous_segment_button: TemplateChild<Button>,
        #[template_child]
        pub previous_frame_button: TemplateChild<Button>,
        #[template_child]
        pub play_button: TemplateChild<Button>,
        #[template_child]
        pub next_frame_button: TemplateChild<Button>,
        #[template_child]
        pub next_segment_button: TemplateChild<Button>,
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
            // self.previous_segment_button.connect_clicked(glib::clone!(
            //     #[strong(rename_to = video_player_container_weak)] self.video_player_container,
            //     #[strong(rename_to = split_table_weak)] self.split_table,
            //     #[weak(rename_to = this)] self,
            //     move |_| {
            //         let video_player_container_borrow = video_player_container_weak.borrow();
            //         let video_player_container_ref = match video_player_container_borrow.as_ref() {
            //             Some(vpc) => vpc,
            //             None => return,
            //         };
            //         let video_player_container = match video_player_container_ref.upgrade() {
            //             Some(vpc) => vpc,
            //             None => return,
            //         };
            //         let split_table_borrow= split_table_weak.borrow();
            //         let split_table_ref = match split_table_borrow.as_ref() {
            //             Some(st) => st,
            //             None => return,
            //         };
            //         let split_table = match split_table_ref.upgrade() {
            //             Some(st) => st,
            //             None => return,
            //         };
            //         let selection_model = split_table.model().and_downcast::<SingleSelection>().unwrap();
            //         let selected_index = selection_model.selected();
            //         let previous_index = selected_index.saturating_sub(1);
            //         selection_model.set_selected(previous_index);
            //         for (video_player_index, child) in flowbox_children(&video_player_container).enumerate() {
            //             let fb_child = match child.downcast_ref::<FlowBoxChild>() {
            //                 Some(c) => c,
            //                 None => continue,
            //             };

            //             let content = match fb_child.child() {
            //                 Some(c) => c,
            //                 None => continue,
            //             };

            //             let video_player = match content.downcast_ref::<VideoPlayer>() {
            //                 Some(vp) => vp,
            //                 None => continue,
            //             };

            //             let arc = match video_player.pipeline().upgrade() {
            //                 Some(a) => a,
            //                 None => {
            //                     eprintln!("Shared jump to segment: Pipeline dropped");
            //                     continue
            //                 }
            //             };

            //             let mut pipeline = match arc.lock() {
            //                 Ok(g) => g,
            //                 Err(_) => {
            //                     eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
            //                     continue
            //                 }
            //             };
                        
            //             if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            //                 let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
            //                 if let Ok(result) = pipeline.seek_position(time) {
            //                     println!("Shared pipeline seek for video player {video_player_index} to position {time}");
            //                 }
            //             }
            //         }
            //         println!("Pressed shared preivous segment button");
            //     }
            // ));
            self.previous_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                move |_| {
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return,
                    };

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
                        
                        pipeline.frame_backward();
                    }
                }
            ));
            self.play_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_liststore_weak)] self.split_table_liststore,
                #[strong(rename_to = scale_start_instant)] self.scale_start_instant,
                #[strong(rename_to = seek_bar)] self.seek_bar,
                #[strong(rename_to = scale_start_offset)] self.scale_start_offset,
                #[strong(rename_to = is_paused)] self.is_paused,
                #[strong(rename_to = sync_manager_weak)] self.sync_manager,
                move |_| {
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let split_table_liststore_borrow = split_table_liststore_weak.borrow();
                    let split_table_liststore_ref = match split_table_liststore_borrow.as_ref() {
                        Some(st) => st,
                        None => return,
                    };
                    let split_table_liststore = match split_table_liststore_ref.upgrade() {
                        Some(st) => st,
                        None => return,
                    };
                    let sync_manager_weak_borrow = sync_manager_weak.borrow();
                    let sync_manager_ref = match sync_manager_weak_borrow.as_ref() {
                        Some(st) => st,
                        None => return,
                    };
                    let sync_manager = match sync_manager_ref.upgrade() {
                        Some(st) => st,
                        None => return,
                    };

                    let mut starts: Vec<ClockTime> = Vec::new();
                    let mut ends: Vec<ClockTime> = Vec::new();
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
                        let last_mark_position = split_table_liststore.item(split_table_liststore.n_items() - 1)
                            .and_downcast::<VideoSegment>()
                            .unwrap()
                            .get_time(video_player_id.to_string().as_str());

                        starts.push(ClockTime::from_seconds(0));
                        ends.push(ClockTime::from_nseconds(last_mark_position));
                    }

                    sync_manager.clear_state();

                    sync_manager.set_on_all_playing(glib::clone!(
                        #[weak(rename_to = start_instant)] scale_start_instant,
                        move || {
                            let now = Instant::now();
                            println!("callbacked now: {now:?}");
                            *start_instant.lock().unwrap() = Some(now);
                        }
                    ));

                    sync_manager.play_videos(starts, ends);

                    scale_start_offset.set(seek_bar.get_scale().value());
                    is_paused.set(!is_paused.get());
                }
            ));
            self.next_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                move |_| {
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return,
                    };
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
                        pipeline.frame_forward();
                    }
                }
            ));
            // self.next_segment_button.connect_clicked(glib::clone!(
            //     #[strong(rename_to = video_player_container_weak)] self.video_player_container,
            //     #[strong(rename_to = split_table_weak)] self.split_table,
            //     move |_| {
            //         let video_player_container_borrow = video_player_container_weak.borrow();
            //         let video_player_container_ref = match video_player_container_borrow.as_ref() {
            //             Some(vpc) => vpc,
            //             None => return,
            //         };
            //         let video_player_container = match video_player_container_ref.upgrade() {
            //             Some(vpc) => vpc,
            //             None => return,
            //         };
            //         let split_table_borrow= split_table_weak.borrow();
            //         let split_table_ref = match split_table_borrow.as_ref() {
            //             Some(st) => st,
            //             None => return,
            //         };
            //         let split_table = match split_table_ref.upgrade() {
            //             Some(st) => st,
            //             None => return,
            //         };
            //         let selection_model = split_table.model().and_downcast::<SingleSelection>().unwrap();
            //         let selected_index = selection_model.selected();
            //         let next_index = (selected_index + 1).clamp(0, selection_model.n_items() - 1);
            //         selection_model.set_selected(next_index);
            //         for (video_player_index, child) in flowbox_children(&video_player_container).enumerate() {
            //             let fb_child = match child.downcast_ref::<FlowBoxChild>() {
            //                 Some(c) => c,
            //                 None => continue,
            //             };

            //             let content = match fb_child.child() {
            //                 Some(c) => c,
            //                 None => continue,
            //             };

            //             let video_player = match content.downcast_ref::<VideoPlayer>() {
            //                 Some(vp) => vp,
            //                 None => continue,
            //             };

            //             let arc = match video_player.pipeline().upgrade() {
            //                 Some(a) => a,
            //                 None => {
            //                     eprintln!("Shared jump to segment: Pipeline dropped");
            //                     continue
            //                 }
            //             };

            //             let mut pipeline = match arc.lock() {
            //                 Ok(g) => g,
            //                 Err(_) => {
            //                     eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
            //                     continue
            //                 }
            //             };
                        
            //             if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
            //                 let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
            //                 if let Ok(result) = pipeline.seek_position(time) {
            //                     println!("Shared pipeline seek for video player {video_player_index} to position {time}");
            //                 }
            //             }
            //         }
            //     }
            // ));
            self.jump_to_segment_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table_column_view,
                move |_| {
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let split_table_borrow= split_table_weak.borrow();
                    let split_table_ref = match split_table_borrow.as_ref() {
                        Some(st) => st,
                        None => return,
                    };
                    let split_table = match split_table_ref.upgrade() {
                        Some(st) => st,
                        None => return,
                    };
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
                        let video_player_id = video_player.get_id();
                        let selection_model = split_table.model().and_downcast::<SingleSelection>().unwrap();
                        if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                            let time = selection.get_time(video_player_id.to_string().as_str());
                            let clock_time = ClockTime::from_nseconds(time);
                            if let Ok(_result) = pipeline.seek_position(clock_time) {
                                println!("Shared pipeline seek for video player {video_player_id} to position {clock_time}");
                            }
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
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return,
                    };
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
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return,
                    };
                    let start_time_offset_liststore_borrow = start_time_offset_liststore_weak.borrow();
                    let start_time_offset_liststore_ref = match start_time_offset_liststore_borrow.as_ref() {
                        Some(st) => st,
                        None => return,
                    };
                    let start_time_offset_liststore = match start_time_offset_liststore_ref.upgrade() {
                        Some(st) => st,
                        None => return,
                    };
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
        imp.setup_buttons();
        imp.setup_seek_bar_control();
        widget.set_controls(false);
        widget
    }

    pub fn connect_row(&self, row_index: u32) {
        let imp = self.imp();
        let split_table_borrow = imp.split_table.borrow();
        let split_table_ref = match split_table_borrow.as_ref() {
            Some(st) => st,
            None => return,
        };
        let split_table= match split_table_ref.upgrade() {
            Some(st) => st,
            None => return,
        };
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore_ref = match split_table_liststore_borrow.as_ref() {
            Some(stl) => stl,
            None => return,
        };
        let split_table_liststore = match split_table_liststore_ref.upgrade() {
            Some(stl) => stl,
            None => return,
        };
        let video_player_container_borrow = imp.video_player_container.borrow();
        let video_player_container_ref = match video_player_container_borrow.as_ref() {
            Some(stl) => stl,
            None => return,
        };
        let video_player_container = match video_player_container_ref.upgrade() {
            Some(stl) => stl,
            None => return,
        };
        
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
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore_ref = match split_table_liststore_borrow.as_ref() {
            Some(stl) => stl,
            None => return,
        };
        let split_table_liststore = match split_table_liststore_ref.upgrade() {
            Some(stl) => stl,
            None => return,
        };
        let split_table_borrow = imp.split_table.borrow();
        let split_table_ref = match split_table_borrow.as_ref() {
            Some(st) => st,
            None => return,
        };
        let split_table = match split_table_ref.upgrade() {
            Some(st) => st,
            None => return,
        };


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
        let video_player_container_borrow = imp.video_player_container.borrow();
        let video_player_container_ref = match video_player_container_borrow.as_ref() {
            Some(vpc) => vpc,
            None => return,
        };
        let video_player_container = match video_player_container_ref.upgrade() {
            Some(vpc) => vpc,
            None => return,
        };
        let split_table_borrow = imp.split_table.borrow();
        let split_table_ref = match split_table_borrow.as_ref() {
            Some(st) => st,
            None => return,
        };
        let split_table = match split_table_ref.upgrade() {
            Some(st) => st,
            None => return,
        };


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
                if let Err(e) = pipeline.seek_position(start_time) {
                    eprintln!("Player {video_player_id} error setting position: {e}");
                }
                imp.seek_bar.get_scale().set_value(0.0);
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
        imp.next_segment_button.set_sensitive(status);
        imp.play_button.set_sensitive(status);
        imp.previous_frame_button.set_sensitive(status);
        imp.previous_segment_button.set_sensitive(status);
    }

    pub fn remove_marks(&self, video_player_id: &str) {
        let imp = self.imp();
        let split_table_liststore_borrow = imp.split_table_liststore.borrow();
        let split_table_liststore_ref = match split_table_liststore_borrow.as_ref() {
            Some(st) => st,
            None => return,
        };
        let split_table_liststore = match split_table_liststore_ref.upgrade() {
            Some(st) => st,
            None => return,
        };

        for i in 0..split_table_liststore.n_items() {
            let segment = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let segment_id = segment.get_segment_id();
            imp.seek_bar.remove_mark(&format!("video-{video_player_id}, segment-{segment_id}"));
        }

        self.update_timeline_length();
    }
}

