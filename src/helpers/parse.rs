use gtk::{Entry};
use glib::{Regex, RegexCompileFlags, RegexMatchFlags};
use gtk::{ glib, prelude::*};

pub fn string_to_nseconds(time: &String) -> Option<u64> {
    let (min, rest) = time.split_once(":").unwrap();
    let (sec, subseconds) = rest.split_once(".").unwrap();

    let minutes = min.parse::<u64>().unwrap();
    let seconds = sec.parse::<u64>().unwrap();

    let nanos = match subseconds.len() {
        0 => 0,
        1 => subseconds.parse::<u64>().unwrap() * 100_000_000, // 0.1s = 100_000_000ns
        2 => subseconds.parse::<u64>().unwrap() * 10_000_000,  // 0.01s = 10_000_000ns
        3 => subseconds.parse::<u64>().unwrap() * 1_000_000,   // 0.001s = 1_000_000ns
        4 => subseconds.parse::<u64>().unwrap() * 100_000,     // 0.0001s
        5 => subseconds.parse::<u64>().unwrap() * 10_000,      // ...
        6 => subseconds.parse::<u64>().unwrap() * 1_000,
        7 => subseconds.parse::<u64>().unwrap() * 100,
        8 => subseconds.parse::<u64>().unwrap() * 10,
        _ => subseconds.parse::<u64>().unwrap() // assume already in nanoseconds
    };
    let total_nanos = minutes * 60 * 1_000_000_000 + seconds * 1_000_000_000 + nanos;
    return Some(total_nanos);
}

pub fn validate_split_table_entry(entry: &Entry) -> bool {
    let input = entry.text().to_string();
    let pattern = r"^[0-5][0-9]:[0-5][0-9]\.\d{3}$";
    // Checks if the input matches the format: MM:SS.sss
    let re = Regex::match_simple(pattern, input.clone(), RegexCompileFlags::empty(), RegexMatchFlags::empty());
    if !re {
        println!("Entry is not in valid format");
    }
    re
}