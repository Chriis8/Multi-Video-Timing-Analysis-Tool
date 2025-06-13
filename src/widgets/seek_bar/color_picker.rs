use std::collections::{HashMap, VecDeque};

#[derive(Clone)]
pub struct ColorPool {
    available: VecDeque<String>,
    in_use: HashMap<String, String>,
}

impl ColorPool {
    pub fn new(colors: Vec<String>) -> Self {
        let color_pool = ColorPool {
            available: VecDeque::from(colors),
            in_use: HashMap::new(),
        };
        color_pool
    }

    pub fn assign_color(&mut self, video_player_id: &str) -> Option<String> {
        if let Some(color) = self.available.pop_front() {
            self.in_use.insert(video_player_id.to_string(), color.to_string());
            return Some(color);
        } else {
            return None;
        }
    }

    pub fn release_color(&mut self, video_player_id: &str) {
        if let Some(color) = self.in_use.remove(video_player_id) {
            self.available.push_back(color);
        }
    }
}

