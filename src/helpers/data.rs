use glib::WeakRef;
use glib::prelude::*;
use gstreamer::message::Error;
use gtk::prelude::ObjectExt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::cell::RefCell;
use std::sync::Weak;
use std::rc::Rc;

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

pub fn store_data<T: 'static>(widget: &impl ObjectExt, key: &str, value: T) {
    unsafe {
        widget.set_data(key, value);
    }
}

pub fn get_data<T: 'static>(widget: &impl ObjectExt, key: &str) -> Option<std::ptr::NonNull<T>> {
    unsafe { widget.data::<T>(key) }
}

pub fn get_next_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn borrow_asref_upgrade<T: ObjectType>(object: &RefCell<Option<WeakRef<T>>>) -> Result<T, String> {
    let object_borrow = object.borrow();
    let object_ref = match object_borrow.as_ref() {
        Some(weak_ref) => weak_ref,
        None => return Err("as_ref error".to_string()),
    };
    let object = match object_ref.upgrade() {
        Some(obj) => obj,
        None => return Err("upgrade failed".to_string()),
    };
    Ok(object)
}

