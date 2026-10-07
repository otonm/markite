//! UI-free editor core. Rules for this crate:
//! - no GUI toolkit types, no threads, no async, no globals;
//! - only `std` types cross the boundary (String, PathBuf, Vec, Result);
//! - every operation is a plain function or method a frontend can call and
//!   mirror into its own property/signal system.

pub mod blocks;
pub mod document;
pub mod location;
mod highlight;
pub mod minimap;
pub mod render;
mod syntax_themes;

pub use document::Document;
