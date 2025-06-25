use std::{cell::RefCell, iter::Once, thread::{self, current}, time::Duration};
use glib::BoolError;
use gstreamer::{event::{Seek, Step}, prelude::*, Clock, ClockTime, Element, Pipeline, SeekFlags, SeekType };
use gtk::{self, Ordering};
use gtk::gdk;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlaybackDirection {
    Forward,
    Reverse,
}

pub struct PipelineState {
    pub direction: PlaybackDirection,
}

pub struct VideoClamp {
    start_time: ClockTime,
    end_time: ClockTime,
}

impl VideoClamp {
    pub fn new(start: ClockTime, end: ClockTime) -> Self {
        VideoClamp { start_time: (start), end_time: (end) }
    }

    pub fn clamp_position(&self, position: ClockTime) -> ClockTime {
        position.min(self.end_time).max(self.start_time)
    }

    pub fn check_and_clamp_position(&self, pipeline: &Pipeline) -> Result<bool, Box<dyn std::error::Error>> {
        if pipeline.current_state() == gstreamer::State::Playing {
            if let Some(position) = pipeline.query_position::<ClockTime>() {
                if position >= self.end_time {
                    println!("clamping end");
                    pipeline.set_state(gstreamer::State::Paused)?;
                    pipeline.state(ClockTime::from_seconds(2)).0?;
                    pipeline.seek_simple(
                        SeekFlags::FLUSH | SeekFlags::ACCURATE, 
                        self.end_time
                    )?;
                    return Ok(true);
                } else if position < self.start_time {
                    println!("clamping start");
                    pipeline.seek_simple(
                        SeekFlags::FLUSH | SeekFlags::ACCURATE, 
                        self.start_time
                    )?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
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
    frame_duration: OnceCell<u64>,
    clamp: Arc<Mutex<Option<VideoClamp>>>,
    monitor_thread: Option<thread::JoinHandle<()>>,
    monitor_active: Arc<AtomicBool>,
}


impl VideoPipeline {
    // Creates new VideoPipeline
    pub fn new() -> Self {
        Self {
            gtksink: gstreamer::ElementFactory::make("gtk4paintablesink").property("sync", true).build().unwrap(),
            pipeline: gstreamer::Pipeline::new(),
            state: RefCell::new(PipelineState::new()),
            frame_duration: OnceCell::new(),
            clamp: Arc::new(Mutex::new(None)),
            monitor_thread: None,
            monitor_active: Arc::new(AtomicBool::new(false)),
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
    pub fn seek_position(&self, position: gstreamer::ClockTime) -> Result<(), Box<dyn std::error::Error>> {
        let duration = self.pipeline.query_duration::<ClockTime>().ok_or("failed to get pipeline position")?;
        if position == duration {
            let frame_time = match self.frame_duration.get() {
                Some(duration) => *duration,
                None => self.set_frame_duration().unwrap(),
            };
            let seek_position = position - ClockTime::from_nseconds(frame_time);
            self.pipeline.seek_simple(gstreamer::SeekFlags::FLUSH, seek_position)?;
            println!("seeked safely to end: (1 frame before): Position: {position}, safe position: {seek_position}");
        } else {
            self.pipeline.seek_simple(gstreamer::SeekFlags::FLUSH, position)?;
            println!("seeked to {position}");
        }
        Ok(())
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

        println!("position_to_percent: position: {position_ns}, duration: {duration_ns}");

        let percent = position_ns as f64 / duration_ns as f64 * 100.0;
        
        Ok(percent)
    }

    pub fn position_to_logical_percent(&self) -> Result<f64, String> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u8 = 20;
        let mut position_opt: Option<ClockTime> = None;
        loop {
            if let Some(pos) = self.pipeline.query_position::<ClockTime>() {
                position_opt = Some(pos);
                break;
            } else {
                attempts += 1;
                println!("attempts at getting position to logical percent");
                if attempts >= MAX_ATTEMPTS {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let position = match position_opt {
            Some(pos) => pos,
            None => {
                eprintln!("Failed to get pipeline position");
                return Err("Failed to get pipeline position".to_string());
            }
        };
        let logical_duration = self.get_logical_duration()?.nseconds();
        let start_time = self.get_start()?;
        let position_ns = position.saturating_sub(start_time).nseconds();

        let percent = position_ns as f64 / logical_duration as f64;
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
        
        let _ = self.pipeline.state(ClockTime::from_seconds(2));

        self.set_frame_duration();
        // let mut attempts = 0;
        // const MAX_ATTEMPTS: u8 = 25;
        // let mut duration_opt: Option<ClockTime> = None;
        // loop {
        //     if let Some(pos) = self.pipeline.query_duration::<ClockTime>() {
        //         duration_opt = Some(pos);
        //         break;
        //     } else {
        //         attempts += 1;
        //         println!("attempts at getting duration after setting up pipeline: {attempts}");
        //         if attempts >= MAX_ATTEMPTS {
        //             break;
        //         }
        //     }
        //     std::thread::sleep(Duration::from_millis(50));
        // }
        // let duration = match duration_opt {
        //     Some(duration) => duration,
        //     None => {
        //         eprintln!("Failed to get pipeline position");
        //         ClockTime::ZERO
        //     }
        // };

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
    pub fn get_bus(&self) -> Option<gstreamer::Bus> {
        self.pipeline.bus()
    }

    // Sets the video to the playing state
    // pub fn play_videox(&self) {
    //     let (_,current_state,_) = self.pipeline.state(gstreamer::ClockTime::NONE);
    //     let new_state = match current_state {
    //         gstreamer::State::Null => return,
    //         gstreamer::State::Playing => gstreamer::State::Paused,
    //         _ => gstreamer::State::Playing,
    //     };

    //     let length = self.pipeline.query_duration::<ClockTime>().unwrap();
    //     let mut state = self.state.borrow_mut();
    //     if new_state == gstreamer::State::Playing && state.direction == PlaybackDirection::Reverse {
    //         self.set_rate(1., ClockTime::ZERO, length);
    //         state.direction = PlaybackDirection::Forward;
    //     }

    //     println!("new state: {:?}", new_state);
    //     self.pipeline.set_state(new_state).expect("Failed to set state");
    // }

    pub fn play_video(&self) {
        let state = self.state.borrow();
        if state.direction == PlaybackDirection::Reverse {
            drop(state);
            self.set_direction_forward();
        }
        println!("new state: Playing");
        self.pipeline.set_state(gstreamer::State::Playing).expect("Failed to set state");
    }

    // Sets the video to the paused state
    pub fn pause_video(&self) {
        println!("new state: Paused");
        self.pipeline.set_state(gstreamer::State::Paused).expect("Failed to set pipeline state to Paused");
    }

    // Sets the video to the Null state
    pub fn stop_video(&self) {
        self.pipeline
            .set_state(gstreamer::State::Null)
            .expect("Failed to set pipeline state to Null");
    }

    pub fn set_direction_forward(&self) {
        let position = match self.pipeline.query_position::<gstreamer::ClockTime>() {
            Some(pos) => pos,
            None => {
                eprintln!("Unable to get current position");
                return;
            }
        };
        let mut state = self.state.borrow_mut();
        //let end_time = state.end.unwrap_or(self.pipeline.query_duration::<ClockTime>().and_then(|clock_time| Some(clock_time.nseconds())).unwrap());
        //let end_time = state.end;
        state.direction = PlaybackDirection::Forward;
        drop(state);
        // let seek_event =
        //     Seek::new(
        //         1.0,
        //         SeekFlags::FLUSH | SeekFlags::ACCURATE,
        //         SeekType::Set,
        //         position,
        //         SeekType::Set,
        //         ClockTime::from_nseconds(end_time),
        //     );
        let seek_event =
            Seek::new(
                1.0,
                SeekFlags::FLUSH | SeekFlags::ACCURATE,
                SeekType::Set,
                position,
                SeekType::End,
                ClockTime::NONE,
            );
        self.pipeline.send_event(seek_event);
    }

    pub fn set_direction_backward(&self) {
        let position = match self.pipeline.query_position::<gstreamer::ClockTime>() {
            Some(pos) => pos,
            None => {
                eprintln!("Unable to get current position");
                return;
            }
        };
        let mut state = self.state.borrow_mut();
        //let start_time = state.start;
        state.direction = PlaybackDirection::Reverse;
        drop(state);
        // let seek_event =
        //     Seek::new(
        //         -1.0,
        //         SeekFlags::FLUSH | SeekFlags::ACCURATE,
        //         SeekType::Set,
        //         ClockTime::from_nseconds(start_time),
        //         SeekType::Set,
        //         position,
        //     );
        let seek_event =
            Seek::new(
                -1.0,
                SeekFlags::FLUSH | SeekFlags::ACCURATE,
                SeekType::Set,
                ClockTime::ZERO,
                SeekType::Set,
                position,
            );
        self.pipeline.send_event(seek_event);
    }

    // Moves video one frame forward
    pub fn frame_forward(&self) {
        if self.pipeline.current_state() != gstreamer::State::Paused {
            eprintln!("Can't step 1 frame forward. Video is not paused");
            return;
        }

        let state = self.state.borrow();
        if state.direction == PlaybackDirection::Reverse {
            drop(state);
            self.set_direction_forward();
        }

        let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
        println!("Attempting to move one frame forward");
        let success = self.pipeline.send_event(step_event);
        if !success {
            eprintln!("Failed to move one frame forward");
        }
    }

    // Moves video one frame backward
    pub fn frame_backward(&self) {
        if self.pipeline.current_state() != gstreamer::State::Paused {
            eprintln!("Can't step 1 frame backward. Video is not paused");
            return;
        }
        // Set video direction backward
        let state = self.state.borrow();
        if state.direction == PlaybackDirection::Forward {
            drop(state);
            self.set_direction_backward();
        }
        // Step 1 frame
        let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
        println!("Attempting to move one frame backward");
        let success = self.pipeline.send_event(step_event);
        if !success {
            eprintln!("Failed to move one frame backward");
        }
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

    // pub fn play_video_clam(&self, start: ClockTime, end: ClockTime) {
    //     let (_,current_state,_) = self.pipeline.state(gstreamer::ClockTime::NONE);
    //     let new_state = match current_state {
    //         gstreamer::State::Null => return,
    //         gstreamer::State::Playing => gstreamer::State::Paused,
    //         _ => gstreamer::State::Playing,
    //     };

    //     let mut state = self.state.borrow_mut();
    //     self.set_rate(1., start, end);
    //     state.direction = PlaybackDirection::Forward;

    //     println!("new state: {:?}", new_state);
    //     self.pipeline.set_state(new_state).expect("Failed to set state");
    // }

    pub fn set_frame_duration(&self) -> Option<u64> {
        let sink = self.pipeline.iterate_sinks().into_iter().find_map(|element| {
            if let Ok(element) = element {
                if element.class().metadata("klass").map_or(false, |klass| klass.contains("Video")) {
                    Some(element)
                } else {
                    eprintln!("5");
                    None
                }
            } else {
                eprintln!("6");
                None
            }
        });

        match sink {
            Some(sink) => {
                if let Some(caps) = sink.static_pad("sink").and_then(|pad| pad.current_caps()) {
                    if let Some(structure) = caps.structure(0) {
                        if let Ok(framerate) = structure.get::<gstreamer::Fraction>("framerate") {
                            let fps = framerate.numer() as f64 / framerate.denom() as f64;
                            let frame_duration_ns = (1_000_000_000.0 / fps) as u64;
                            let _ = self.frame_duration.set(frame_duration_ns);
                            return Some(frame_duration_ns);
                        } else {
                            eprintln!("1");
                        }
                    } else {
                        eprintln!("2");
                    }
                } else {
                    eprintln!("3");
                }
            }
            None => { eprintln!("4")}
        };
        None
    }

    pub fn pipeline(&self) -> Option<Pipeline> {
        return Some(self.pipeline.clone());
    }

    pub fn get_logical_duration(&self) -> Result<ClockTime, String> {
        if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            Ok(clamp.end_time - clamp.start_time)
        } else {
            Err("Video is not currently clamped".to_string())
        }
    }

    pub fn get_start(&self) -> Result<ClockTime, String> {
        if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            Ok(clamp.start_time)
        } else {
            Err("Video not is currently clamped".to_string())
        }
    }

    pub fn get_end(&self) -> Result<ClockTime, String> {
        if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            Ok(clamp.end_time)
        } else {
            Err("Video not is currently clamped".to_string())
        }
    }

    // pub fn set_start_clamp(&self, start_time: u64) {
    //     let mut state = self.state.borrow_mut();
    //     state.start = start_time;
    // }

    // pub fn set_end_clamp(&self, end_time: u64) {
    //     let mut state = self.state.borrow_mut();
    //     state.end = end_time;
    // }

    pub fn reset_clamps(&mut self) -> Result<(), String> {
        *self.clamp.lock().unwrap() = None;
        self.stop_position_monitor();
        Ok(())
        // let mut state = self.state.borrow_mut();
        // state.start = 0;
        // let length = self.pipeline.query_duration::<gstreamer::format::Time>().and_then(|clocktime| Some(clocktime.nseconds())).unwrap();
        // state.end = length;
    }

    pub fn apply_clamp(&mut self, start: ClockTime, end: ClockTime) -> Result<(), String> {
        if start > end {
            return Err("start exceeds end clamp".to_string());
        }
        println!("clamping start: {start}, end: {end}");
        let clamp = VideoClamp::new(start, end);
        
        *self.clamp.lock().unwrap() = Some(clamp);

        if !self.monitor_active.load(std::sync::atomic::Ordering::Relaxed) {
            self.start_position_monitor();
        }

        self.seek_to_start()?;
        Ok(())
    }

    fn seek_to_start(&self) -> Result<(), String> {
        let position = if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            clamp.start_time.clone()
        } else {
            ClockTime::ZERO
        };
        self.seek_clamped(position)
            .map_err(|e| format!("Failed to see to start: {e}"))
        // if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
        //     let start = clamp.start_time.clone();
        //     // let _ = drop(clamp);
        //     self.seek_clamped(start)
        //         .map_err(|e| format!("Failed to seek to start: {e}"))
        // } else {
        //     self.seek_clamped(ClockTime::ZERO)
        //         .map_err(|e| format!("Failed to see to start: {e}"))
        // }
    }

    fn seek_to_end(&self) -> Result<(), String> {
        let position = if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            clamp.end_time.clone()
        } else if let Some(duration) = self.pipeline.query_duration::<ClockTime>() {
            duration
        } else {
            Err("Could not get video duration".to_string())?
        };
        self.seek_clamped(position)
            .map_err(|e| format!("Failed to seek to end: {e}"))
        
        // if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
        //     self.seek_clamped(clamp.end_time)
        //         .map_err(|e| format!("Failed to see to end: {e}"))
        // } else if let Some(duration) = self.pipeline.query_duration::<ClockTime>() {
        //     self.seek_clamped(duration)
        //         .map_err(|e| format!("Failed to seek to end: {e}"))
        // } else {
        //     Err("Could not get video duration".to_string())
        // }
    }

    pub fn seek_clamped(&self, position: ClockTime) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            let clamped_pos = clamp.clamp_position(position);
            self.seek_position(clamped_pos)?
            // self.pipeline.seek_simple(
            //     SeekFlags::FLUSH | SeekFlags::ACCURATE, 
            //     clamped_pos)?
        } else {
            self.seek_position(position)?
        }
        Ok(())
        // self.pipeline.seek_simple(
        //         SeekFlags::FLUSH | SeekFlags::ACCURATE, 
        //         position)
    }

    pub fn frame_forward_clamped(&self) -> Result<bool, String> {
        if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
            if let Some(position) = self.pipeline.query_position::<ClockTime>() {
                let next_position = position + ClockTime::from_nseconds(*self.frame_duration.get().unwrap());
                if next_position > clamp.end_time {
                    return Ok(false);
                }

            }
        }
        
        if self.pipeline.current_state() != gstreamer::State::Paused {
            eprintln!("Can't step 1 frame forward. Video is not paused");
            return Ok(false);
        }

        let state = self.state.borrow();
        if state.direction == PlaybackDirection::Reverse {
            drop(state);
            self.set_direction_forward();
        }

        let step_event = Step::new(gstreamer::format::Buffers::ONE, 1.0, true, false);
        println!("Attempting to move one frame forward");
        if self.pipeline.send_event(step_event) {
            Ok(true)
        } else {
            eprintln!("Failed to move one frame forward");
            Err("failed to send step event".to_string())
        }
    }

    pub fn frame_backward_clamped(&self) -> Result<bool, String> {
        if let Some(position) = self.pipeline.query_position::<ClockTime>() {
            let prev_position = position.saturating_sub(ClockTime::from_nseconds(*self.frame_duration.get().unwrap()));
            if let Some(clamp) = self.clamp.lock().unwrap().as_ref() {
                if prev_position < clamp.start_time {
                    return Ok(false);
                }
            }
            
            if self.pipeline.current_state() != gstreamer::State::Paused {
                eprintln!("Can't step 1 frame forward. Video is not paused");
                return Ok(false);
            }
    
            self.seek_clamped(prev_position)
                .map_err(|e| format!("Failed to step backward: {e}"))?;
            Ok(true)
        } else {
            Err("Could not get video position".to_string())
        }
    }

    fn start_position_monitor(&mut self) {
        if self.monitor_active.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        self.monitor_active.store(true,std::sync::atomic::Ordering::Relaxed);

        let pipeline_weak = self.pipeline.downgrade();
        let clamp_ref = Arc::clone(&self.clamp);
        let active_flag = Arc::clone(&self.monitor_active);
        
        self.monitor_thread = Some(thread::spawn(move || {
            let mut last_position = ClockTime::ZERO;

            while active_flag.load(std::sync::atomic::Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));

                if let Some(pipeline) = pipeline_weak.upgrade() {
                    if let Some(clamp) = clamp_ref.lock().unwrap().as_ref() {
                        if let Ok(was_clamped) = clamp.check_and_clamp_position(&pipeline) {
                            if was_clamped {
                                if let Some(current_pos) = pipeline.query_position::<ClockTime>() {
                                    if current_pos <= clamp.start_time && last_position > clamp.start_time {
                                        println!("Start boundary reached, position clamped");
                                    } else if current_pos >= clamp.end_time && last_position < clamp.end_time {
                                        println!("End boundary reached, video paused and position clamped");
                                    }
                                    last_position = current_pos;
                                }
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }));
    }

    fn stop_position_monitor(&mut self) {
        self.monitor_active.store(false, std::sync::atomic::Ordering::Relaxed);

        if let Some(handle) = self.monitor_thread.take() {
            let _ = handle.join();
        }
    }

}

impl Default for VideoPipeline {
    fn default() -> Self {
        Self::new()
    }
}