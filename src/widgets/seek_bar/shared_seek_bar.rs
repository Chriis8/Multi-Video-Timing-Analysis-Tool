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
use gstreamer::ClockTime;
use std::cell::{RefCell, Cell};
use std::rc::Rc;
use glib::WeakRef;
use crate::helpers::ui::flowbox_children;

mod imp {
    use std::time::Duration;

    use glib::timeout_add_local;

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
        pub split_table: RefCell<Option<WeakRef<ColumnView>>>,
        pub start_time_offset_table: RefCell<Option<WeakRef<ListStore>>>,
        pub split_table_liststore: RefCell<Option<WeakRef<ListStore>>>,

        pub is_dragging: Rc<Cell<bool>>,
        pub has_control: Rc<Cell<bool>>,
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
            self.previous_segment_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
                #[weak(rename_to = this)] self,
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
                    let selection_model = split_table.model().and_downcast::<SingleSelection>().unwrap();
                    let selected_index = selection_model.selected();
                    let previous_index = selected_index.saturating_sub(1);
                    selection_model.set_selected(previous_index);
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

                        let mut guard = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        if let Some(pipeline) = guard.as_mut() {
                            if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                                let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
                                if let Ok(result) = pipeline.seek_position(time) {
                                    println!("Shared pipeline seek for video player {video_player_index} to position {time}");
                                }
                            }
                        } else {
                            eprintln!("No pipeline for index {video_player_index}");
                        }
                    }
                    println!("Pressed shared preivous segment button");
                }
            ));
            self.previous_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
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

                        let mut guard = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        if let Some(pipeline) = guard.as_mut() {
                            pipeline.frame_backward();
                        } else {
                            eprintln!("No pipeline for index {video_player_index}");
                        }
                    }
                }
            ));
            self.play_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
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
                    let split_table_borrow = split_table_weak.borrow();
                    let split_table_ref = match split_table_borrow.as_ref() {
                        Some(st) => st,
                        None => return,
                    };
                    let split_table = match split_table_ref.upgrade() {
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

                        let mut guard = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        if let Some(pipeline) = guard.as_mut() {
                            pipeline.play_video();
                        } else {
                            eprintln!("No pipeline for index {video_player_index}");
                        }
                    }
                }
            ));
            self.next_frame_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
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

                        let mut guard = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        if let Some(pipeline) = guard.as_mut() {
                            pipeline.frame_forward();
                        } else {
                            eprintln!("No pipeline for index {video_player_index}");
                        }
                    }
                }
            ));
            self.next_segment_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
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
                    let selection_model = split_table.model().and_downcast::<SingleSelection>().unwrap();
                    let selected_index = selection_model.selected();
                    let next_index = (selected_index + 1).clamp(0, selection_model.n_items() - 1);
                    selection_model.set_selected(next_index);
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

                        let mut guard = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        if let Some(pipeline) = guard.as_mut() {
                            if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                                let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
                                if let Ok(result) = pipeline.seek_position(time) {
                                    println!("Shared pipeline seek for video player {video_player_index} to position {time}");
                                }
                            }
                        } else {
                            eprintln!("No pipeline for index {video_player_index}");
                        }
                    }
                }
            ));
            self.jump_to_segment_button.connect_clicked(glib::clone!(
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = split_table_weak)] self.split_table,
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

                        let mut guard = match arc.lock() {
                            Ok(g) => g,
                            Err(_) => {
                                eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                                continue
                            }
                        };

                        if let Some(pipeline) = guard.as_mut() {
                            let selection_model = split_table.model().and_downcast::<SingleSelection>().unwrap();
                            if let Some(selection) = selection_model.selected_item().and_downcast::<VideoSegment>() {
                                let time = selection.get_time(video_player_index).and_then(|nanos| Some(ClockTime::from_nseconds(nanos))).unwrap();
                                if let Ok(result) = pipeline.seek_position(time) {
                                    println!("Shared pipeline seek for video player {video_player_index} to position {time}");
                                }
                            }
                        } else {
                            eprintln!("No pipeline for index {video_player_index}");
                        }
                    }
                }
            ));
        }

        pub fn setup_seek_bar_control(&self) {
            let shared_scale = self.seek_bar.get_scale();
            let _ = timeout_add_local(Duration:: from_millis(5000), glib::clone!(
                #[strong(rename_to = is_dragging)] self.is_dragging,
                #[strong(rename_to = has_control)] self.has_control,
                #[strong(rename_to = video_player_container_weak)] self.video_player_container,
                #[strong(rename_to = start_time_offset_table_weak)] self.start_time_offset_table,
                #[strong(rename_to = shared_scale)] shared_scale,
                move || {
                    if is_dragging.get() || !has_control.get() {
                        println!("doesnt have control. skipping");
                        return glib::ControlFlow::Continue;
                    }
                    println!("HAS CONTROLLL WOOOOOT");
                    let video_player_container_borrow = video_player_container_weak.borrow();
                    let video_player_container_ref = match video_player_container_borrow.as_ref() {
                        Some(vpc) => vpc,
                        None => return glib::ControlFlow::Continue,
                    };
                    let video_player_container = match video_player_container_ref.upgrade() {
                        Some(vpc) => vpc,
                        None => return glib::ControlFlow::Continue,
                    };
                    let start_time_offset_table_borrow = start_time_offset_table_weak.borrow();
                    let start_time_offset_table_ref = match start_time_offset_table_borrow.as_ref() {
                        Some(st) => st,
                        None => return glib::ControlFlow::Continue,
                    };
                    let start_time_offset_table = match start_time_offset_table_ref.upgrade() {
                        Some(st) => st,
                        None => return glib::ControlFlow::Continue,
                    };

                    let video_player = video_player_container
                        .first_child()
                        .and_downcast::<FlowBoxChild>()
                        .unwrap()
                        .child()
                        .and_downcast::<VideoPlayer>()
                        .unwrap();
                    let arc = match video_player.pipeline().upgrade() {
                        Some(a) => a,
                        None => {
                            eprintln!("Shared jump to segment: Pipeline dropped");
                            return glib::ControlFlow::Continue;
                        }
                    };

                    let mut guard = match arc.lock() {
                        Ok(g) => g,
                        Err(_) => {
                            eprintln!("Shared jump to segment: Failed to lock pipeline mutex");
                            return glib::ControlFlow::Continue;
                        }
                    };

                    if let Some(pipeline) = guard.as_mut() {
                        let offset_time_entry = start_time_offset_table.item(0).and_downcast::<TimeEntry>().unwrap();
                        let offset_time = offset_time_entry.get_time();
                        let current_position = match pipeline.get_position() {
                            Some(pos) => pos,
                            None => return glib::ControlFlow::Continue,
                        };
                        let video_length = match pipeline.get_length() {
                            Some(length) => length,
                            None => return glib::ControlFlow::Continue,
                        };
                        let new_value_percent = (current_position.nseconds() - offset_time) as f64 / video_length as f64;
                        shared_scale.set_value(new_value_percent * 100.0);
                    }
                    glib::ControlFlow::Continue
                }
            ));
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
    pub fn new(video_player_container: &FlowBox, split_table: &ColumnView, start_time_offset_table: &ListStore, split_table_liststore: &ListStore) -> Self {
        let widget: Self = glib::Object::new::<Self>();
        let imp = imp::SharedSeekBar::from_obj(&widget);
        imp.seek_bar.set_can_target(false);
        imp.seek_bar.set_can_focus(false);
        imp.seek_bar.set_auto_timeline_length_handling(true);
        imp.video_player_container.borrow_mut().replace(Downgrade::downgrade(video_player_container));
        imp.split_table.borrow_mut().replace(Downgrade::downgrade(split_table));
        imp.start_time_offset_table.borrow_mut().replace(Downgrade::downgrade(start_time_offset_table));
        imp.split_table_liststore.borrow_mut().replace(Downgrade::downgrade(split_table_liststore));
        imp.setup_buttons();
        imp.setup_seek_bar_control();
        widget
    }

    pub fn connect_row(&self, row_index: u32, video_player_count: u32) {
        let imp = self.imp();
        let start_time_offset_table_borrow = imp.start_time_offset_table.borrow();
        let start_time_offset_table_ref = match start_time_offset_table_borrow.as_ref() {
            Some(stot) => stot,
            None => return,
        };
        let start_time_offset_table = match start_time_offset_table_ref.upgrade() {
            Some(stot) => stot,
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
        
        let row = split_table_liststore.item(row_index).and_downcast::<VideoSegment>().unwrap();
        let row_count = split_table_liststore.n_items();
        let colors = vec!["red", "blue", "green", "black", "coral", "lavender"];
        for i in 0..video_player_count {
            let time = row.get_time_entry_copy(i as usize);
            let offset = start_time_offset_table.item(i as u32).and_downcast::<TimeEntry>().unwrap();  
            // id should always be row_count regardless of if the row is inserted in the middle.
            // not sure if it will matter but this should give marks unique ids
            let row_id = row_count - 1; 
            imp.seek_bar.add_mark(format!("video-{i}, row-{row_id}"), time, colors[i as usize], offset);
        }
    }

    pub fn connect_column(&self, column_index: u32, video_player_count: u32) {
        let imp = self.imp();
        let start_time_offset_table_borrow = imp.start_time_offset_table.borrow();
        let start_time_offset_table_ref = match start_time_offset_table_borrow.as_ref() {
            Some(stot) => stot,
            None => return,
        };
        let start_time_offset_table = match start_time_offset_table_ref.upgrade() {
            Some(stot) => stot,
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

        let row_count = split_table_liststore.n_items();
        let colors = vec!["red", "blue", "green", "black", "coral", "lavender"];
        for i in 0..row_count {
            let row = split_table_liststore.item(i).and_downcast::<VideoSegment>().unwrap();
            let time = row.get_time_entry_copy(column_index as usize);
            let offset = start_time_offset_table.item((video_player_count as u32).saturating_sub(1)).and_downcast::<TimeEntry>().unwrap();
            // id are given in order as they have already been created
            imp.seek_bar.add_mark(format!("video-{column_index}, seg-{i}"), time, colors[(video_player_count - 1) as usize], offset);
        }
    }

    pub fn update_timeline_length(self) {
        let imp = self.imp();
        imp.seek_bar.update_timeline_length();
    }

    pub fn toggle_has_control(&self) {
        let imp = self.imp();
        let status = imp.has_control.get();
        imp.has_control.set(!status);
    }
}

