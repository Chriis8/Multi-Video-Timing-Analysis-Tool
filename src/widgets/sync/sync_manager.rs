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
        pub shared_clock: OnceCell<Clock>,
        pub sync_callbacks: RefCell<Vec<Box<dyn Fn(SyncEvent)>>>,
        pub is_synced: Rc<Cell<bool>>,
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for SyncManager {
        const NAME: &'static str = "SyncManager";
        type Type = super::SyncManager;
    }

    impl SyncManager {
    }

    impl ObjectImpl for SyncManager {
        fn dispose(&self) {
            println!("Disposing sync manager");
            self.pipelines.lock().unwrap().clear();
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
        let _ = imp.shared_clock.set(gstreamer::SystemClock::obtain().upcast());
        object
    }

    //Adds pipeline to sync manager
    pub fn add_pipeline(&self, pipeline_id: &str, video_pipeline_weak: Weak<Mutex<VideoPipeline>>) -> Result<(), Box<dyn std::error::Error>> {
        let mut imp = self.imp();
        let video_pipeline_arc = match video_pipeline_weak.upgrade() {
            Some(p) => p,
            None => return Err("Failed to upgrade video pipeline".into()),
        };
        let video_pipeline = video_pipeline_arc.lock().unwrap();
        println!("Adding pipeline into sync manager");
        imp.pipelines.lock().unwrap().insert(pipeline_id.to_string(), video_pipeline_weak);
        Ok(())

    }

    //Removes pipeline from sync manager
    pub fn remove_pipeline(&self, pipeline_id: &str) {
        let imp = self.imp();
        imp.pipelines.lock().unwrap().remove(pipeline_id);
    }

    //Syncs and plays each video
    //Input: Start time offsets for each individual video
    //Output: Shared base time for the videos and the progression of the videos to display on the ui
    pub fn play_videos(&self, offsets: HashMap<String, u64>) {
        let imp = self.imp();
        if imp.is_synced.get() {
            return;
        }
        //Get time from the central clock to use for base times
        let shared_clock_time = imp.shared_clock.get().unwrap().time().unwrap();

        for (video_player_id, pipeline_weak) in imp.pipelines.lock().unwrap().iter() {
            let video_pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };

            let pipeline = video_pipeline.lock().unwrap();
            let offset_time = offsets.get(video_player_id).unwrap();

            //Sync each pipeline to the same central clock 
            pipeline.pipeline().unwrap().use_clock(imp.shared_clock.get());

            //Updates base time relative to the start time offset past in
            pipeline.pipeline().unwrap().set_base_time(shared_clock_time + ClockTime::from_nseconds(*offset_time));
            
            pipeline.play_video();
        }

        //Finds the position the ui progression bar should be.
        let scale_position = self.get_current_logical_position();
        self.emit_event(SyncEvent::PlaybackStarted { base_time: shared_clock_time, scale_position: scale_position });
    }

    //Pauses all videos
    pub fn pause_videos(&self) {
        let imp = self.imp();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            pipeline.lock().unwrap().pause_video();
        }

        self.emit_event(SyncEvent::PlaybackPaused);
    }

    //Moves each video 1 frame forward
    pub fn frame_forward(&self) {
        let imp = self.imp();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            //Moves pipeline forward a frame with respect to the clamp times
            let result = pipeline.lock().unwrap().frame_forward_clamped();
            if let Err(e) = result {
                eprintln!("Sync manager frame forward error: {e}");
            }
        }
    }

    //Move each video 1 frame backward
    pub fn frame_backward(&self) {
        let imp = self.imp();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            //Moves pipeline backward a frame with respect to the clamp times
            let result = pipeline.lock().unwrap().frame_backward_clamped();
            if let Err(e) = result {
                eprintln!("Sync manager frame backward error: {e}");
            }
        }
    }

    //Set each video to specified position
    //Input: HashMap<video_player_id, position>
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
            
            //Sync each pipeline to the same central clock 
            pipeline.pipeline().unwrap().use_clock(imp.shared_clock.get());
            
            //Updates base time to relect the new position
            pipeline.pipeline().unwrap().set_base_time(shared_clock_time - *position);
            
            //Performs the seek opertation with respect the clamped times
            let result = pipeline.seek_clamped(*position);
            if let Err(e) = result {
                eprintln!("Failed to perform seek clamped: {e}");
            }
        }
        self.emit_event(SyncEvent::Seeked);
    }

    //Not currently being used
    pub fn sync_clocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();
        let sync_time = imp.shared_clock.get().unwrap().time().unwrap();
        self.emit_event(SyncEvent::SyncEnabled { base_time: sync_time });
        Ok(())
    }

    //Restores individual clocks to the pipelines
    pub fn unsync_clocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();
        self.pause_videos();

        //Wait for each video that finish switching to paused state
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            if let Some(video_pipeline) = pipeline_weak.upgrade() {
                let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
                pipeline.state(ClockTime::from_seconds(5)).0?;
            }
        }

        //Record current position before unsyncing
        let mut positions: HashMap<String, ClockTime> = HashMap::new();
        for (id, pipeline_weak) in imp.pipelines.lock().unwrap().iter() {
            if let Some(video_pipeline) = pipeline_weak.upgrade() {
                let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
                if let Some(position) = pipeline.query_position::<ClockTime>() {
                    positions.insert(id.clone(), position);
                }
            }
        }

        //Removes shared clock from pipelines
        for (id, pipeline_weak)  in imp.pipelines.lock().unwrap().iter() {
            if let Some(video_pipeline) = pipeline_weak.upgrade() {
                let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
                println!("RESETING CLOCK");
                
                //reset internal state and wait for state change
                let _ = pipeline.set_state(gstreamer::State::Null);
                let _ = pipeline.state(ClockTime::from_seconds(1)).0?;

                //allow gstreamer to manager new clock
                pipeline.auto_clock();
                
                //set to pause and wait for state change
                let _ = pipeline.set_state(gstreamer::State::Paused);
                let _ = pipeline.state(ClockTime::from_seconds(1)).0?;
                
                //restore position from before unsyncing
                if let Some(clock) = pipeline.clock() {
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

    fn get_current_logical_position(&self) -> ClockTime {
        if let Some(video_pipeline_arc) = self.get_longest_pipeline() {
            let video_pipeline = video_pipeline_arc.lock().unwrap();
            let duration = video_pipeline.get_logical_duration().unwrap().nseconds();
            let percent = video_pipeline.position_to_logical_percent().ok().unwrap();
            let logical_position = (percent * duration as f64) as u64;
            return ClockTime::from_nseconds(logical_position);
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


}
