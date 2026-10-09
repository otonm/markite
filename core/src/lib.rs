//! UI-free editor core. Rules for this crate:
//! - no GUI toolkit types, no threads, no async, no mutable globals;
//! - only `std` types cross the boundary (String, PathBuf, Vec, Result);
//! - every operation is a plain function or method a frontend can call and
//!   mirror into its own property/signal system;
//! - inputs from the frontend are validated here (non-finite numbers, out-of-range indices), never trusted.

pub mod blocks;
pub mod document;
pub mod encoding;
mod highlight;
pub mod location;
pub mod minimap;
pub mod options;
mod render;
mod syntax_themes;
#[doc(hidden)]
pub mod trace;

pub use document::Document;
