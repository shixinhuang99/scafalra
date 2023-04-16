#[macro_export]
macro_rules! print_flush {
    ($($arg: tt)*) => {
        print!($($arg)*);
        std::io::stdout().flush()?;
    };
}
