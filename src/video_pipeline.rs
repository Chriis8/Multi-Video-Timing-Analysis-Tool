use std::{cell::RefCell, time::Duration};
use gstreamer::{event::{Seek, Step}, prelude::*, ClockTime, CoreError, ErrorMessage, SeekFlags, SeekType, StateChangeReturn, StateChangeSuccess};
use gtk;
use gtk::gdk;

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
    gtksink: gstreamer::Element,
    pipeline: gstreamer::Pipeline,
    state: RefCell<PipelineState>,
}


impl VideoPipeline {
    // Creates new VideoPipeline
    pub fn new() -> Self {
        Self {
            gtksink: gstreamer::ElementFactory::make("gtk4paintablesink").property("sync", true).build().unwrap(),
            pipeline: gstreamer::Pipeline::new(),
            state: RefCell::new(PipelineState::new()),
        }
    }

    // Sets state to NULL to be cleaned up
    pub fn cleanup(&mut self) {
        self.pipeline.set_state(gstreamer::State::Null).unwrap();
    }

    // Resets object to default values
    pub fn reset(&mut self) {
        self.cleanup();        
        *self = Self::new();
    }

    // Updates rate of video playback
    //  1.0 - forward
    // -1.0 - backward
    fn set_rate(&self, rate: f64, start: ClockTime, end: ClockTime) -> bool {
        let position = match self.pipeline.query_position::<gstreamer::ClockTime>() {
            Some(pos) => pos,
            None => {
                eprintln!("Unable to get current position");
                return false;
            }
        };
        let seek_event = if rate > 0. {
            Seek::new(
                rate,
                SeekFlags::FLUSH | SeekFlags::ACCURATE,
                SeekType::Set,
                position,
                SeekType::Set,
                end,
            )
        } else {
            Seek::new(
                rate,
                SeekFlags::FLUSH | SeekFlags::ACCURATE,
                SeekType::Set,
                start,
                SeekType::Set,
                position,
            )
        };
        self.pipeline.send_event(seek_event);
        true
    }

    // Sets video playback to inputted ClockTime
    pub fn seek_position(&self, position: gstreamer::ClockTime) -> Result<(), glib::BoolError> {
        self.pipeline.seek_simple(gstreamer::SeekFlags::FLUSH, position)
    }

    // Retrieves the percent complete the video playback is at
    pub fn position_to_percent(&self) -> Result<f64, glib::Error> {
        let position = match self.pipeline.query_position::<gstreamer::ClockTime>() {
            Some(pos) => pos,
            None => {
                eprintln!("Failed to get pipeline position");
                return Err(glib::Error::new(glib::FileError::Failed, "Failed to get pipeline position"));
            }
        };

        let total_duration = match self.pipeline.query_duration::<gstreamer::ClockTime>() {
            Some(dur) => dur,
            None => {
                eprintln!("Unable to get current duration");
                return Err(glib::Error::new(glib::FileError::Failed, "Unable to get pipeline duration"));
            }
        };

        let position_ns = position.nseconds();
        let duration_ns = total_duration.nseconds();

        let percent = position_ns as f64 / duration_ns as f64 * 100.0;
        
        Ok(percent)
    }

    // Retrieves the position as the percentage of the total duration of the video playback
    pub fn percent_to_position(&self, percent: f64) -> Result<u64, glib::Error> {
        let total_duration = match self.pipeline.query_duration::<gstreamer::ClockTime>() {
            Some(dur) => dur.nseconds(),
            None => {
                eprintln!("Unable to get current duration");
                return Err(glib::Error::new(glib::FileError::Failed, "Unable to get pipeline duration"));
            }
        };

        println!("Duration: {total_duration}");
        println!("Percent: {percent}");

        let new_position = (total_duration as f64 * percent) as u64;
        Ok(new_position)
    }

    // Sets up video pipeline
    pub fn build_pipeline(&self, uri: Option<&str>) {
        let uri = uri.unwrap();
        println!("building pipeline from {uri}");
        
        // Sets up pipeline elements
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


        // Connects elements in pipeline
        self.pipeline.add_many([&source, &audio_convert, &audio_resample, &audio_sink, &video_convert, &video_rate, &video_scale, &self.gtksink]).unwrap();
        gstreamer::Element::link_many([&audio_convert, &audio_resample, &audio_sink])
            .expect("Failed to link audio elements");
        gstreamer::Element::link_many([&video_convert, &video_rate, &video_scale, &self.gtksink])
            .expect("Failed to link video elements");

        let audio_convert_weak = audio_convert.downgrade();
        let video_convert_weak = video_convert.downgrade();
        
        // Connects source pads to video and audio sink
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

            // Links audio pad
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
            
            // Link video pad
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
        self.pipeline
            .set_state(gstreamer::State::Paused)
            .expect("Failed to set pipeline state to paused");
        
    }

    // Returns paintable object for gtk widget
    pub fn get_paintable(&self) -> gdk::Paintable {
        self.gtksink.property::<gdk::Paintable>("paintable")
    }

    // Returns videos current position in ClockTime
    pub fn get_position(&self) -> Option<gstreamer::ClockTime> {
        self.pipeline.query_position::<gstreamer::ClockTime>()    
    }

    // Gets video bus
    pub fn get_bus(&self) -> gstreamer::Bus {
        self.pipeline.bus().unwrap()
    }

    // Sets the video to the playing state
    pub fn play_video(&self) {
        let (_,current_state,_) = self.pipeline.state(gstreamer::ClockTime::NONE);
        let new_state = match current_state {
            gstreamer::State::Null => return,
            gstreamer::State::Playing => gstreamer::State::Paused,
            _ => gstreamer::State::Playing,
        };

        let length = self.pipeline.query_duration::<ClockTime>().unwrap();
        let mut state = self.state.borrow_mut();
        if new_state == gstreamer::State::Playing && state.direction == PlaybackDirection::Reverse {
            self.set_rate(1., ClockTime::ZERO, length);
            state.direction = PlaybackDirection::Forward;
        }

        println!("new state: {:?}", new_state);
        self.pipeline.set_state(new_state).expect("Failed to set state");
    }

    // Sets the video to the paused state
    pub fn pause_video(&self) {
        self.pipeline
            .set_state(gstreamer::State::Paused)
            .expect("Failed to set pipeline state to Paused");
    }

    // Sets the video to the Null state
    pub fn stop_video(&self) {
        self.pipeline
        .set_state(gstreamer::State::Null)
        .expect("Failed to set pipeline state to Null");
}

    // Moves video one frame forward
    pub fn frame_forward(&self) {
        eprintln!("---------- (Frame forwards Before) video time: {:?}", self.pipeline.query_position::<gstreamer::ClockTime>());
        if self.pipeline.current_state() != gstreamer::State::Paused {
            eprintln!("Can't step 1 frame forward. Video is not paused");
            return;
        }
        // Sets direction to forward
        let length = self.pipeline.query_duration::<ClockTime>().unwrap();
        let mut state = self.state.borrow_mut();
        if state.direction == PlaybackDirection::Reverse {
            self.set_rate(1., ClockTime::ZERO, length);
            state.direction = PlaybackDirection::Forward;
        }
        // Steps 1 frame
        let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
        println!("Attempting to move one frame forward");
        let success = self.pipeline.send_event(step_event);
        if !success {
            eprintln!("Failed to move one frame forward");
        }
        eprintln!("---------- (Frame forwards After) video time: {:?}", self.pipeline.query_position::<gstreamer::ClockTime>());
    }

    // Moves video one frame backward
    pub fn frame_backward(&self) {
        eprintln!("---------- (Frame backwards Before) video time: {:?}", self.pipeline.query_position::<gstreamer::ClockTime>());
        if self.pipeline.current_state() != gstreamer::State::Paused {
            eprintln!("Can't step 1 frame backward. Video is not paused");
            return;
        }
        // Set video direction backward
        let length = self.pipeline.query_duration::<ClockTime>().unwrap();
        let mut state = self.state.borrow_mut();
        if state.direction == PlaybackDirection::Forward {
            self.set_rate(-1., ClockTime::ZERO, length);
            state.direction = PlaybackDirection::Reverse;
        }
        // Step 1 frame
        let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
        println!("Attempting to move one frame backward");
        let success = self.pipeline.send_event(step_event);
        if !success {
            eprintln!("Failed to move one frame backward");
        }
        eprintln!("---------- (Frame backwards After) video time: {:?}", self.pipeline.query_position::<gstreamer::ClockTime>());
    }

    // Prints the currect frame the video is on
    pub fn get_current_frame(&self) {
        let current_time = self.pipeline.query_position::<gstreamer::format::Time>().unwrap();
        
        let video_sink = self.pipeline.by_name("video_convert").unwrap();
        let sink_pads = video_sink.static_pad("sink").unwrap();
        let caps = sink_pads.current_caps().unwrap();
        let structure = caps.structure(0).unwrap();
        if let Ok(fps_fraction) = structure.get::<gstreamer::Fraction>("framerate") {
            let fps = fps_fraction.numer();
            let time_per_frame = 1_000_000_000 / fps as u64;
            let current_frame = current_time.nseconds() / time_per_frame;
            println!("Current_frame: {current_frame}");
        } else {
            println!("Can't get the framerate");
        }
    }

    pub fn get_length(&self) -> Option<u64> {
        match self.pipeline.state(Some(ClockTime::from_seconds(5))) {
            (Ok(_state_change_success), _, _) => {
                if let Some(duration) = self.pipeline.query_duration::<ClockTime>() {
                    println!("Got duration from get_length");
                    Some(duration.nseconds())
                } else {
                    println!("Didnt get duration but state was success");
                    let _ = self.pipeline.set_state(gstreamer::State::Null);
                    None
                }
            }
            _ => {
                println!("State change not successful");
                let _ = self.pipeline.set_state(gstreamer::State::Null);
                None
            }
        }
    }

    pub fn play_video_clamp(&self, start: ClockTime, end: ClockTime) {
        let (_,current_state,_) = self.pipeline.state(gstreamer::ClockTime::NONE);
        let new_state = match current_state {
            gstreamer::State::Null => return,
            gstreamer::State::Playing => gstreamer::State::Paused,
            _ => gstreamer::State::Playing,
        };

        let mut state = self.state.borrow_mut();
        self.set_rate(1., start, end);
        state.direction = PlaybackDirection::Forward;

        println!("new state: {:?}", new_state);
        self.pipeline.set_state(new_state).expect("Failed to set state");
    }

}