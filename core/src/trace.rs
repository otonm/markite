//! Debug tracing. With the `trace` feature on, `trace!` prints a timestamped line to stderr; without it the
//! macro expands to `()`, so the call, its arguments and its format strings are not in the binary at all
//! (release builds). Frontends use it as `markite_core::trace!`. Never log document text, only sizes/paths.

#[cfg(feature = "trace")]
#[doc(hidden)]
pub fn emit(module: &str, args: std::fmt::Arguments) {
    use std::{sync::OnceLock, time::Instant};
    static START: OnceLock<Instant> = OnceLock::new();
    let ms = START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0;
    eprintln!("[trace {ms:10.3}ms {module}] {args}");
}

#[cfg(feature = "trace")]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { $crate::trace::emit(module_path!(), format_args!($($arg)*)) };
}

#[cfg(not(feature = "trace"))]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { () };
}
