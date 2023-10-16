use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG: AtomicBool = AtomicBool::new(false);

pub fn trun_on_debug(val: bool) {
	DEBUG.store(val, Ordering::Relaxed);
}

pub fn is_debug_mode() -> bool {
	DEBUG.load(Ordering::Relaxed)
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        if $crate::debug::is_debug_mode() {
            println!($($arg)*);
        }
    }};
}
