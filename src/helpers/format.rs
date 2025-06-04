use gstreamer::ClockTime;

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