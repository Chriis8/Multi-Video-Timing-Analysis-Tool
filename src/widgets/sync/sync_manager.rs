use gstreamer::prelude::{ClockExt, ElementExt, PipelineExt};
use gstreamer::{MessageView, SeekFlags};
use gtk::glib;
use glib::subclass::Signal;
use once_cell::sync::Lazy;
use gtk::subclass::{prelude::*};
use gtk::prelude::*;
use crate::video_pipeline::VideoPipeline;
use std::sync::{Weak, Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use gstreamer::ClockTime;
use glib::Object;
use gstreamer::bus::BusWatchGuard;
use gstreamer::Clock;
use gstreamer::prelude::*;
use std::cell::{RefCell, Cell, OnceCell};
use std::rc::Rc;
use crate::widgets::split_panel::timeentry::TimeEntry;
use std::thread;

#[derive(Clone, Debug)]
pub enum SyncEvent {
    SyncEnabled { base_time: ClockTime },
    SyncDisabled,
    PlaybackStarted { base_time: ClockTime, scale_position: ClockTime },
    PlaybackPaused,
    Seeked,
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SyncManager {
        pub pipelines: Arc<Mutex<HashMap<String, Weak<Mutex<VideoPipeline>>>>>,
        //pub playing_pipelines: Arc<Mutex<HashSet<String>>>,
        //pub on_all_playing: Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>,
        //pub buses: Arc<Mutex<HashMap<String, BusWatchGuard>>>,
        pub shared_clock: OnceCell<Clock>,
        pub sync_callbacks: RefCell<Vec<Box<dyn Fn(SyncEvent)>>>,
        pub is_synced: Rc<Cell<bool>>,
        //pub current_base_time: RefCell<Option<ClockTime>>,
        //pub video_fps: RefCell<HashMap<String, u64>>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for SyncManager {
        const NAME: &'static str = "SyncManager";
        type Type = super::SyncManager;
    }

    impl SyncManager {
    }

    impl ObjectImpl for SyncManager {
        // fn signals() -> &'static [Signal] {
        //     // Setup split button signal
        //     static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
        //         vec![Signal::builder("all-videos-playing")
        //             .flags(glib::SignalFlags::RUN_LAST)
        //             .build(),]
        //         });
        //     SIGNALS.as_ref()
        // }

        fn dispose(&self) {
            println!("Disposing sync manager");
            self.pipelines.lock().unwrap().clear();
            //self.playing_pipelines.lock().unwrap().clear();
            //self.on_all_playing.lock().unwrap().take();
            //self.buses.lock().unwrap().clear();
        }
    }
}

glib::wrapper! {
    pub struct SyncManager(ObjectSubclass<imp::SyncManager>)
    @implements gtk::Buildable;
}

impl SyncManager {
    pub fn new() -> Self {
        let object: Self = glib::Object::new::<Self>();
        let imp = imp::SyncManager::from_obj(&object);
        //*imp.on_all_playing.lock().unwrap() = None;
        let _ = imp.shared_clock.set(gstreamer::SystemClock::obtain().upcast());
        object
    }

    pub fn add_pipeline(&self, pipeline_id: &str, video_pipeline_weak: Weak<Mutex<VideoPipeline>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut imp = self.imp();
        let video_pipeline_arc = match video_pipeline_weak.upgrade() {
            Some(p) => p,
            None => return Err("Failed to upgrade video pipeline".into()),
        };
        let video_pipeline = video_pipeline_arc.lock().unwrap();
        let bus = video_pipeline.get_bus().unwrap();
        let name = pipeline_id.to_string();
        
        //let fps = pipeline_lock.set_frame_duration().unwrap();
        //imp.video_fps.borrow_mut().insert(pipeline_id.to_string(), fps);
        // let bus_source_id = bus.add_watch_local(glib::clone!(
        //     #[strong(rename_to = name)] name,
        //     #[strong(rename_to = this)] self,
        //     #[strong(rename_to = pipeline)] pipeline_arc,
        //     move |_, msg| {
        //         match msg.view() {
        //             MessageView::StateChanged(state) => {
        //                 let gst_pipeline = pipeline.lock().unwrap().pipeline().unwrap();
        //                 if msg.src().map(|s| s == gst_pipeline.upcast_ref::<Object>()).unwrap_or(false) {
        //                     if state.current() == gstreamer::State::Playing {
        //                         this.mark_pipeline_playing(name.as_str());
        //                     }
        //                 }

        //             }
        //             _ => {}
        //         }
        //         glib::ControlFlow::Continue
        //     }
        // )).expect(&format!("failed to add bus watch for {pipeline_id}"));
        println!("Adding pipeline into sync manager");
        imp.pipelines.lock().unwrap().insert(pipeline_id.to_string(), video_pipeline_weak);
        Ok(())
        //imp.buses.lock().unwrap().insert(pipeline_id.to_string(), bus_source_id);

    }

    pub fn remove_pipeline(&self, pipeline_id: &str) {
        let imp = self.imp();
        imp.pipelines.lock().unwrap().remove(pipeline_id);
        //imp.playing_pipelines.lock().unwrap().remove(pipeline_id);
        //imp.buses.lock().unwrap().remove(pipeline_id);
    }

    pub fn play_videos(&self, offsets: HashMap<String, u64>) {
        let imp = self.imp();
        if imp.is_synced.get() {
            return;
        }
        let shared_clock_time = imp.shared_clock.get().unwrap().time().unwrap();
        // let new_base_time = imp.shared_clock.get().unwrap().time().unwrap();
        // self.apply_base_time_to_all(new_base_time);

        for (video_player_id, pipeline_weak) in imp.pipelines.lock().unwrap().iter() {
            let video_pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };

            let pipeline = video_pipeline.lock().unwrap();
            pipeline.pipeline().unwrap().use_clock(imp.shared_clock.get());
            let offset_time = offsets.get(video_player_id).unwrap();
            pipeline.pipeline().unwrap().set_base_time(shared_clock_time + ClockTime::from_nseconds(*offset_time));
            pipeline.play_video();
        }

        let scale_position = self.get_current_logical_position();
        self.emit_event(SyncEvent::PlaybackStarted { base_time: shared_clock_time, scale_position: scale_position });
    }

    pub fn pause_videos(&self) {
        let imp = self.imp();

        //let current_media_time = self.get_current_media_time().ok().unwrap();
        //self.clear_state();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            pipeline.lock().unwrap().pause_video();
        }

        self.emit_event(SyncEvent::PlaybackPaused);
    }

    pub fn frame_forward(&self) {
        let imp = self.imp();
        //self.clear_state();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            let result = pipeline.lock().unwrap().frame_forward_clamped();
            if let Err(e) = result {
                eprintln!("Sync manager frame forward error: {e}");
            }
        }
    }

    pub fn frame_backward(&self) {
        let imp = self.imp();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            let result = pipeline.lock().unwrap().frame_backward_clamped();
            if let Err(e) = result {
                eprintln!("Sync manager frame backward error: {e}");
            }
        }

        // //self.clear_state();
        // let current_media_time = self.get_current_media_time().ok().unwrap();
        // let frame_duration = *imp.video_fps.borrow().values().min().unwrap();

        // let target_position = current_media_time.saturating_sub(ClockTime::from_nseconds(frame_duration));

        // for pipeline_weak in imp.pipelines.lock().unwrap().values() {
        //     let pipeline = match pipeline_weak.upgrade() {
        //         Some(p) => p,
        //         None => return,
        //     };
        //     pipeline.lock().unwrap().pause_video();
        // }

        // for pipeline_weak in imp.pipelines.lock().unwrap().values() {
        //     let pipeline = match pipeline_weak.upgrade() {
        //         Some(p) => p,
        //         None => return,
        //     };
        //     let _ = pipeline.lock().unwrap().pipeline().unwrap().state(ClockTime::from_seconds(1)).0;
        // }

        // self.seek(target_position);

    }

    pub fn seek(&self, positions: HashMap<String, ClockTime>) {
        let imp = self.imp();
        let shared_clock_time = imp.shared_clock.get().unwrap().time().unwrap();
        for (video_player_id, pipeline_weak) in imp.pipelines.lock().unwrap().iter() {
            let video_pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            let position = positions.get(video_player_id.as_str()).unwrap();
            let pipeline = video_pipeline.lock().unwrap();
            pipeline.pipeline().unwrap().use_clock(imp.shared_clock.get());
            pipeline.pipeline().unwrap().set_base_time(shared_clock_time - *position);
            
            let result = pipeline.seek_clamped(*position);
            if let Err(e) = result {
                eprintln!("Failed to perform seek clamped: {e}");
            }

        }

        // let new_base_time = imp.shared_clock.get().unwrap().time().unwrap() - position;
        // self.apply_base_time_to_all(new_base_time);
        //let scale_position = self.get_current_logical_position();
        self.emit_event(SyncEvent::Seeked);
    }

    // pub fn mark_pipeline_playing(&self, pipeline_id: &str) {
    //     let imp = self.imp();
    //     let mut playing = imp.playing_pipelines.lock().unwrap();
    //     println!("marking pipeline as playing");
    //     if !playing.insert(pipeline_id.to_string()) {
    //         println!("couldnt insert pipeline as playing");
    //         return;
    //     }
    //     let pipelines = imp.pipelines.lock().unwrap();
    //     if pipelines.len() == playing.len() {
    //         println!("All pipeline playing");
    //         if let Some(callback) = imp.on_all_playing.lock().unwrap().take() {
    //             println!("calling callback function");
    //             callback();
    //         }
    //     }
    // }

    // pub fn clear_state(&self) {
    //     let imp = self.imp();
    //     imp.playing_pipelines.lock().unwrap().clear();
    //     imp.on_all_playing.lock().unwrap().take();
    // }

    // pub fn set_on_all_playing<F>(&self, f: F) where F: FnOnce() + Send + 'static, {
    //     let imp = self.imp();
    //     let mut callback = imp.on_all_playing.lock().unwrap();
    //     println!("Set callback");
    //     *callback = Some(Box::new(f));
    // }

    pub fn sync_clocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();

        // for pipeline_weak in imp.pipelines.lock().unwrap().values() {
        //     let video_pipeline = pipeline_weak.upgrade().unwrap();
        //     let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
        //     pipeline.state(ClockTime::from_seconds(5)).0?;
        // }
        
        let sync_time = imp.shared_clock.get().unwrap().time().unwrap();
        // for pipeline_weak in imp.pipelines.lock().unwrap().values() {
        //     let video_pipeline = pipeline_weak.upgrade().unwrap();
        //     let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
        //     pipeline.use_clock(imp.shared_clock.get());
        //     pipeline.set_base_time(sync_time);
        // }
        self.emit_event(SyncEvent::SyncEnabled { base_time: sync_time });
        Ok(())
    }

    pub fn unsync_clocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();
        self.pause_videos();

        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            if let Some(video_pipeline) = pipeline_weak.upgrade() {
                let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
                pipeline.state(ClockTime::from_seconds(5)).0?;
            }
        }

        let mut positions: HashMap<String, ClockTime> = HashMap::new();

        for (id, pipeline_weak) in imp.pipelines.lock().unwrap().iter() {
            if let Some(video_pipeline) = pipeline_weak.upgrade() {
                let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
                if let Some(position) = pipeline.query_position::<ClockTime>() {
                    positions.insert(id.clone(), position);
                }
            }
        }

        for (id, pipeline_weak)  in imp.pipelines.lock().unwrap().iter() {
            if let Some(video_pipeline) = pipeline_weak.upgrade() {
                let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
                println!("RESETING CLOCK");
                //let original_timing_state = original_timing_states.remove(id).unwrap();
                let _ = pipeline.set_state(gstreamer::State::Null);
                let _ = pipeline.state(ClockTime::from_seconds(1)).0?;

                pipeline.auto_clock();
                
                let _ = pipeline.set_state(gstreamer::State::Paused);
                let _ = pipeline.state(ClockTime::from_seconds(1)).0?;
                
                //pipeline.use_clock(original_timing_state.clock.as_ref());
                //pipeline.use_clock(None::<&Clock>);
                if let Some(clock) = pipeline.clock() {
                    //pipeline.set_base_time(clock.time().unwrap());
                    //pipeline.set_base_time(original_timing_state.base_time.unwrap());
                    if let Some(&position) = positions.get(id) {
                        pipeline.seek_simple(
                            SeekFlags::FLUSH | SeekFlags::ACCURATE, 
                            position)?;
                    } 
                }
            }
        }
        self.emit_event(SyncEvent::SyncDisabled);
        Ok(())
    }

    pub fn get_shared_clock(&self) -> Clock {
        let imp = self.imp();
        imp.shared_clock.get().unwrap().clone()
    } 

    pub fn add_sync_callback<F>(&self, callback: F) 
    where F: Fn(SyncEvent) + 'static {
        let imp = self.imp();
        imp.sync_callbacks.borrow_mut().push(Box::new(callback));
    }

    fn emit_event(&self, event: SyncEvent) {
        let imp = self.imp();
        for callback in imp.sync_callbacks.borrow().iter() {
            callback(event.clone());
        }
    }

    // fn get_current_media_time(&self) -> Result<ClockTime, Box<dyn std::error::Error>> {
    //     let imp = self.imp();
    //     if let Some(base_time) = imp.current_base_time.borrow().as_ref() {
    //         let clock_time = imp.shared_clock.get().unwrap().time().unwrap();
    //         Ok(clock_time - *base_time)
    //     } else {
    //         Err("No base time set".into())
    //     }
    // }

    fn apply_base_time_to_all(&self, base_time: ClockTime) {
        let imp = self.imp();

        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
            pipeline.use_clock(imp.shared_clock.get());
            pipeline.set_base_time(base_time);
        }
        //*imp.current_base_time.borrow_mut() = Some(base_time);
    }

    fn get_current_logical_position(&self) -> ClockTime {
        if let Some(video_pipeline_arc) = self.get_longest_pipeline() {
            let video_pipeline = video_pipeline_arc.lock().unwrap();
            let duration = video_pipeline.get_logical_duration().unwrap().nseconds();
            let percent = video_pipeline.position_to_logical_percent().ok().unwrap();
            let logical_position = (percent * duration as f64) as u64;
            return ClockTime::from_nseconds(logical_position);
            // if let Some(position) = pipeline.query_position::<ClockTime>() {
            //     let percent = (position - ClockTime::from_nseconds(video_pipeline.get_start())) / ClockTime::from_nseconds(duration);
            //     return ClockTime::from_nseconds(percent);
            // }
        }
        ClockTime::ZERO
    }

    fn get_longest_pipeline(&self) -> Option<Arc<Mutex<VideoPipeline>>> {
        let imp = self.imp();
        let mut longest_pipeline: Option<Arc<Mutex<VideoPipeline>>> = None; 
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline_arc = pipeline_weak.upgrade().unwrap();
            let video_pipeline = video_pipeline_arc.lock().unwrap();
            
            let duration = video_pipeline.get_logical_duration();
            if let Some(longest_pipeline_arc) = longest_pipeline.clone() {
                if duration > longest_pipeline_arc.lock().unwrap().get_logical_duration() {
                    longest_pipeline = Some(video_pipeline_arc.clone());
                }
            } else {
                longest_pipeline = Some(video_pipeline_arc.clone());
            }
        }
        longest_pipeline
    }

    pub fn fix_clock(&self) {
        let imp = self.imp();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();

            let _ = pipeline.set_state(gstreamer::State::Null);
            let _ = pipeline.state(ClockTime::from_seconds(3)).0;
            
            pipeline.auto_clock();
            // let _ = pipeline.set_state(gstreamer::State::Ready);
            // let _ = pipeline.state(ClockTime::from_seconds(3)).0;

            //self.reset_all_element_base_times(&pipeline);

            let _ = pipeline.set_state(gstreamer::State::Paused);
            let _ = pipeline.state(ClockTime::from_seconds(3)).0;
        }
    }

    pub fn break_clock(&self) {
        let imp = self.imp();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();

            pipeline.use_clock(Clock::NONE);
        }
    }


}
