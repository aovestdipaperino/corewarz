//! Terminal visualizer and debugger for CoreWarz.
//!
//! The visualizer drives a [`corewarz_core::Mars`] through the public API and
//! observes it through [`corewarz_core::Observer`]. It cannot corrupt a
//! battle, and every VM improvement shows up here for free.

pub mod app;
pub mod render;
pub mod state;

pub use app::App;
pub use render::render;
pub use state::VisualState;
