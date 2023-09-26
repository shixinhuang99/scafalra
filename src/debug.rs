use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG: AtomicBool = AtomicBool::new(false);

pub fn set_debug(val: bool) {
	DEBUG.store(val, Ordering::Relaxed);
}

pub fn get_debug() -> bool {
	DEBUG.load(Ordering::Relaxed)
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        if $crate::debug::get_debug() {
            println!($($arg)*);
        }
    }};
}
