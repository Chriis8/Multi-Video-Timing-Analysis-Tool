use std::cell::RefCell;
use gstreamer::{event::{self, Seek, Step}, prelude::*, Element, SeekFlags, SeekType};
use gtk;
use gtk::{gdk, glib};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlaybackDirection {
    Forward,
    Reverse,
}

pub struct PipelineState {
    pub direction: PlaybackDirection,
}

impl PipelineState {
    pub fn new() -> Self {
        PipelineState {
            direction: PlaybackDirection::Forward,
        }
    }
}
pub struct VideoPipeline {
    gtksink: Option<glib::WeakRef<gstreamer::Element>>,
    pipeline: Option<glib::WeakRef<gstreamer::Pipeline>>,
    state: RefCell<PipelineState>
}


impl VideoPipeline {
    pub fn new(gtksink: gstreamer::Element, pipeline: gstreamer::Pipeline) -> Self {
        Self {
            gtksink: Some(gtksink.downgrade()),
            pipeline: Some(pipeline.downgrade()),
            state: RefCell::new(PipelineState::new()),
        }
    }

    fn with_pipeline<F, R>(&self, func: F) -> Option<R> where F: FnOnce (&gstreamer::Pipeline) -> R, {
        self.pipeline
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .map(|pipeline| func(&pipeline))
    }

    fn with_gtksink<F, R>(&self, func: F) -> Option<R> where F: FnOnce (&gstreamer::Element) -> R, {
        self.gtksink
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .map(|gtksink| func(&gtksink))
    }

    fn with_pipeline_and_gtksink<F, R>(&self, func: F) -> Option<R> where F: FnOnce (&gstreamer::Pipeline, &gstreamer::Element) -> R, {
        self.pipeline
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .and_then(|pipeline| {
                self.gtksink
                    .as_ref()
                    .and_then(|weak_gtksink| weak_gtksink.upgrade())
                    .map(|gtksink| func(&pipeline, &gtksink))

            })
    }

    fn send_seek_event(&self, rate: f64) -> Option<bool> {
        self.with_pipeline(|pipeline| {
            let position = match pipeline.query_position::<gstreamer::ClockTime>() {
                Some(pos) => pos,
                None => {
                    eprintln!("Unable to get current position");
                    return false;
                }
            };
            let seek_event = if rate > 0. {
                Seek::new(
                    rate,
                    SeekFlags::FLUSH | SeekFlags::KEY_UNIT | SeekFlags::ACCURATE,
                    SeekType::Set,
                    position,
                    SeekType::End,
                    gstreamer::ClockTime::ZERO,
                )
            } else {
                Seek::new(
                    rate,
                    SeekFlags::FLUSH | SeekFlags::ACCURATE,
                    SeekType::Set,
                    gstreamer::ClockTime::ZERO,
                    SeekType::Set,
                    position,
                )
            };
            pipeline.send_event(seek_event)
        })
    }

    pub fn build_pipeline(&self, uri: Option<&str>) {
        let uri = uri.unwrap();
        println!("building pipeline from {uri}");
        
        let source = gstreamer::ElementFactory::make("uridecodebin")
            .name("source")
            .property("uri", uri)
            .build()
            .expect("Failed to build source element");
        let audio_convert = gstreamer::ElementFactory::make("audioconvert")
            .name("audio_convert")
            .build()
            .expect("Failed to build audioconvert element");
        let audio_resample = gstreamer::ElementFactory::make("audioresample")
            .name("audio_resample")
            .build()
            .expect("Failed to build audio resampler element");
        let audio_sink = gstreamer::ElementFactory::make("autoaudiosink")
            .name("audio_sink")
            .build()
            .expect("Failed to build audiosink element");
        let video_convert = gstreamer::ElementFactory::make("videoconvert")
            .name("video_convert")
            .build()
            .expect("Failed to vuild video convert element");
        let video_rate = gstreamer::ElementFactory::make("videorate")
            .name("video_rate")
            .build()
            .expect("Failed to build video rate elements");
        let video_scale = gstreamer::ElementFactory::make("videoscale")
            .name("video_scale")
            .build()
            .expect("Failed to build video scale element");

        self.with_pipeline_and_gtksink(|pipeline, gtksink| {
            pipeline.add_many([&source, &audio_convert, &audio_resample, &audio_sink, &video_convert, &video_rate, &video_scale, &gtksink]).unwrap();
            
            gstreamer::Element::link_many([&audio_convert, &audio_resample, &audio_sink])
            .expect("Failed to link audio elements");
        
            gstreamer::Element::link_many([&video_convert, &video_rate, &video_scale, &gtksink])
            .expect("Failed to link video elements");
    
        });

        let audio_convert_weak = audio_convert.downgrade();
        let video_convert_weak = video_convert.downgrade();

        source.connect_pad_added(move |src, src_pad| {
            println!("Recieved new pad {} from {}", src_pad.name(), src.name());

            let audio_convert = match audio_convert_weak.upgrade() {
                Some(audio_convert) => audio_convert,
                None => {
                    println!("Audio convert element has been dropped");
                    return;
                }
            };

            let video_convert = match video_convert_weak.upgrade() {
                Some(video_convert) => video_convert,
                None => {
                    println!("Video convert element has been dropped");
                    return;
                }
            };

            let has_caps = src_pad.current_caps().is_some();
            let pad_type = src_pad.current_caps()
                .expect("Failed to get caps of new pad")
                .structure(0)
                .expect("Failed to get first strcuture")
                .name();

            if let Some(audio_sink_pad) = audio_convert.static_pad("sink") {
                if audio_sink_pad.is_linked() {
                    println!("Audio pad is already linked. Ignoring");
                    return;
                }
                if has_caps && src_pad.link(&audio_sink_pad).is_ok() {
                    println!("{} pad linked successfully!", pad_type);
                } else {
                    println!("Failed to link {} to audio pad", pad_type);
                }
            }
        
            if let Some(video_sink_pad) = video_convert.static_pad("sink") {
                if video_sink_pad.is_linked() {
                    println!("Video pad is already linked. Ignoring");
                    return;
                }
                if has_caps && src_pad.link(&video_sink_pad).is_ok() {
                    println!("{} pad linked successfully!", pad_type);
                } else {
                    println!("Failed to link {} to video pad", pad_type);
                }
            }
        });
        println!("pipeline built");
        self.with_pipeline(|pipeline| {
            pipeline
                .set_state(gstreamer::State::Paused)
                .expect("Failed to set pipeline state to paused");
        });
        
    }

    pub fn get_paintable(&self) -> Option<gdk::Paintable> {
        self.with_gtksink(|gtksink| {
            gtksink.property::<gdk::Paintable>("paintable")
        })
    }

    pub fn get_position(&self) -> Option<gstreamer::ClockTime> {
        self.with_pipeline(|pipeline| {
            pipeline.query_position::<gstreamer::ClockTime>()    
        }).unwrap()
    }

    pub fn get_bus(&self) -> Option<gstreamer::Bus> {
        self.with_pipeline(|pipeline| {
            pipeline.bus()
        }).unwrap()
    }

    pub fn play_video(&self) {
        self.with_pipeline(|pipeline| {
            let (success,current_state,_) = pipeline.state(gstreamer::ClockTime::NONE);
            let new_state = match current_state {
                gstreamer::State::Null => return,
                gstreamer::State::Playing => gstreamer::State::Paused,
                _ => gstreamer::State::Playing,
            };

            let mut state = self.state.borrow_mut();
            if new_state == gstreamer::State::Playing && state.direction == PlaybackDirection::Reverse {
                self.send_seek_event(1.)
                    .expect("Could not set playback direction to forward");
                state.direction = PlaybackDirection::Forward;
            }

            println!("new state: {:?}", new_state);
            pipeline.set_state(new_state).expect("Failed to set state");
        });
    }

    pub fn pause_video(&self) {
        self.with_pipeline(|pipeline| {
            pipeline
            .set_state(gstreamer::State::Paused)
            .expect("Failed to set pipeline state to Paused");
        });
    }

    pub fn stop_video(&self) {
        self.with_pipeline(|pipeline| {
            pipeline
                .set_state(gstreamer::State::Null)
                .expect("Failed to set pipeline state to Null"); 
        });
    }

    pub fn frame_forward(&self) {
        self.with_pipeline(|pipeline| {
            if pipeline.current_state() != gstreamer::State::Paused {
                eprintln!("Can't step 1 frame forward. Video is not paused");
                return;
            }
            let mut state = self.state.borrow_mut();
            if state.direction == PlaybackDirection::Reverse {
                self.send_seek_event(1.)
                    .expect("Could not set playback direction to forward");
                state.direction = PlaybackDirection::Forward;
            }
            let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
            println!("Attempting to move one frame forward");
            let success = pipeline.send_event(step_event);
            if !success {
                eprintln!("Failed to move one frame forward");
            }
        });
    }

    pub fn frame_backward(&self) {
        self.with_pipeline(|pipeline| {
            if pipeline.current_state() != gstreamer::State::Paused {
                eprintln!("Can't step 1 frame backward. Video is not paused");
                return;
            }
            let mut state = self.state.borrow_mut();
            if state.direction == PlaybackDirection::Forward {
                self.send_seek_event(-1.)
                    .expect("Could not set playback diretion to backward");
                state.direction = PlaybackDirection::Reverse;
            }
            let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
            println!("Attempting to move one frame backward");
            let success = pipeline.send_event(step_event);
            if !success {
                eprintln!("Failed to move one frame backward");
            }
        });
    }
}