use gtk::prelude::ObjectExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn store_data<T: 'static>(widget: &impl ObjectExt, key: &str, value: T) {
    unsafe {
        widget.set_data(key, value);
    }
}

pub fn get_data<T: 'static>(widget: &impl ObjectExt, key: &str) -> Option<std::ptr::NonNull<T>> {
    unsafe { widget.data::<T>(key) }
}

pub fn get_next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

