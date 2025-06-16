use gstreamer::prelude::{ClockExt, ElementExt, PipelineExt};
use gstreamer::MessageView;
use gtk::glib;
use glib::subclass::Signal;
use once_cell::sync::Lazy;
use gtk::subclass::{prelude::*};
use gtk::prelude::*;
use crate::video_pipeline::VideoPipeline;
use std::sync::{Weak, Arc, Mutex};
use std::collections::{HashMap, HashSet};
use gstreamer::ClockTime;
use glib::Object;
use gstreamer::bus::BusWatchGuard;
use gstreamer::Clock;
use std::cell::OnceCell;


mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SyncManager {
        pub pipelines: Arc<Mutex<HashMap<String, Weak<Mutex<VideoPipeline>>>>>,
        pub playing_pipelines: Arc<Mutex<HashSet<String>>>,
        pub on_all_playing: Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>,
        pub buses: Arc<Mutex<HashMap<String, BusWatchGuard>>>,
        pub shared_clock: OnceCell<Clock>
    }
    
    #[gtk::glib::object_subclass]
    impl ObjectSubclass for SyncManager {
        const NAME: &'static str = "SyncManager";
        type Type = super::SyncManager;
    }

    impl SyncManager {
    }

    impl ObjectImpl for SyncManager {
        fn signals() -> &'static [Signal] {
            // Setup split button signal
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("all-videos-playing")
                    .flags(glib::SignalFlags::RUN_LAST)
                    .build(),]
                });
            SIGNALS.as_ref()
        }

        fn dispose(&self) {
            println!("Disposing sync manager");
            self.pipelines.lock().unwrap().clear();
            self.playing_pipelines.lock().unwrap().clear();
            self.on_all_playing.lock().unwrap().take();
            self.buses.lock().unwrap().clear();
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
        *imp.on_all_playing.lock().unwrap() = None;
        imp.shared_clock.set(gstreamer::SystemClock::obtain().upcast());
        object
    }

    pub fn add_pipeline(&self, pipeline_id: &str, pipeline: Weak<Mutex<VideoPipeline>>) {
        let imp = self.imp();
        let pipeline_arc = match pipeline.upgrade() {
            Some(p) => p,
            None => return,
        };
        let pipeline_lock = pipeline_arc.lock().unwrap();
        let bus = pipeline_lock.get_bus().unwrap();
        let name = pipeline_id.to_string();
        let bus_source_id = bus.add_watch_local(glib::clone!(
            #[strong(rename_to = name)] name,
            #[strong(rename_to = this)] self,
            #[strong(rename_to = pipeline)] pipeline_arc,
            move |_, msg| {
                match msg.view() {
                    MessageView::StateChanged(state) => {
                        let gst_pipeline = pipeline.lock().unwrap().pipeline().unwrap();
                        if msg.src().map(|s| s == gst_pipeline.upcast_ref::<Object>()).unwrap_or(false) {
                            if state.current() == gstreamer::State::Playing {
                                this.mark_pipeline_playing(name.as_str());
                            }
                        }

                    }
                    _ => {}
                }
                glib::ControlFlow::Continue
            }
        )).expect(&format!("failed to add bus watch for {pipeline_id}"));
        println!("Adding pipeline into sync manager");
        imp.pipelines.lock().unwrap().insert(pipeline_id.to_string(), pipeline);
        imp.buses.lock().unwrap().insert(pipeline_id.to_string(), bus_source_id);

    }

    pub fn remove_pipeline(&self, pipeline_id: &str) {
        let imp = self.imp();
        imp.pipelines.lock().unwrap().remove(pipeline_id);
        imp.playing_pipelines.lock().unwrap().remove(pipeline_id);
        imp.buses.lock().unwrap().remove(pipeline_id);
    }

    pub fn play_videos(&self) {
        let imp = self.imp();
        for (i, pipeline_weak) in imp.pipelines.lock().unwrap().iter() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            pipeline.lock().unwrap().play_video();
        }
    }

    pub fn pause_videos(&self) {
        let imp = self.imp();
        self.clear_state();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            pipeline.lock().unwrap().pause_video();
        }
    }

    pub fn frame_forward(&self) {
        let imp = self.imp();
        self.clear_state();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            pipeline.lock().unwrap().frame_forward();
        }
    }

    pub fn frame_background(&self) {
        let imp = self.imp();
        self.clear_state();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let pipeline = match pipeline_weak.upgrade() {
                Some(p) => p,
                None => return,
            };
            pipeline.lock().unwrap().frame_backward();
        }
    }

    pub fn mark_pipeline_playing(&self, pipeline_id: &str) {
        let imp = self.imp();
        let mut playing = imp.playing_pipelines.lock().unwrap();
        println!("marking pipeline as playing");
        if !playing.insert(pipeline_id.to_string()) {
            println!("couldnt insert pipeline as playing");
            return;
        }
        let pipelines = imp.pipelines.lock().unwrap();
        if pipelines.len() == playing.len() {
            println!("All pipeline playing");
            if let Some(callback) = imp.on_all_playing.lock().unwrap().take() {
                println!("calling callback function");
                callback();
            }
        }
    }

    pub fn clear_state(&self) {
        let imp = self.imp();
        imp.playing_pipelines.lock().unwrap().clear();
        imp.on_all_playing.lock().unwrap().take();
    }

    pub fn set_on_all_playing<F>(&self, f: F) where F: FnOnce() + Send + 'static, {
        let imp = self.imp();
        let mut callback = imp.on_all_playing.lock().unwrap();
        println!("Set callback");
        *callback = Some(Box::new(f));
    }

    pub fn sync_clocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();

        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
            pipeline.state(ClockTime::from_seconds(5)).0?;
        }
        
        let sync_time = imp.shared_clock.get().unwrap().time().unwrap();
        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
            pipeline.use_clock(imp.shared_clock.get());
            pipeline.set_base_time(sync_time);
        }
        Ok(())
    }

    pub fn unsync_clocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let imp = self.imp();

        for pipeline_weak in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
            pipeline.state(ClockTime::from_seconds(5)).0?;
        }

        for pipeline_weak  in imp.pipelines.lock().unwrap().values() {
            let video_pipeline = pipeline_weak.upgrade().unwrap();
            let pipeline = video_pipeline.lock().unwrap().pipeline().unwrap();
            pipeline.use_clock(None::<&Clock>);
            if let Some(clock) = pipeline.clock() {
                pipeline.set_base_time(clock.time().unwrap());
            }
        }
        Ok(())
    }
}
