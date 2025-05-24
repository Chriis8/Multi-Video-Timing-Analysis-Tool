use crate::widgets::split_panel::splits::VideoSegment;
use gstreamer::ClockTime;
use gtk::prelude::*;
use gio::ListStore;


pub fn print_vec(model: &ListStore) {
    println!("Splits");
    for i in 0..model.n_items() {
        print!("Row: {i} ");
        if let Some(item) = model.item(i).and_downcast::<VideoSegment>() {
            for j in 0..item.get_segment_count() {
                let time = item.get_time(j).unwrap();
                let duration = item.get_duration(j).unwrap();
                print!("{time}, {duration} |");
            }
        }
        println!("");
    }
}

pub fn format_clock(time: u64) -> String {
    if time == u64::MAX {
        return String::new();
    }
    let mut ret = ClockTime::from_nseconds(time).to_string();
    let hours_offset = ret.find(":").unwrap();
    let hour= ret[..hours_offset].to_string();
    let hour_parsed: u32 = hour.parse().unwrap();
    if hour_parsed == 0 {
        ret.drain(..hours_offset+1);
    }
    let split = ret.find(".").unwrap();
    let digits_after_decimal_point = 3;
    ret.truncate(split + digits_after_decimal_point + 1);
    ret
}