//! Language frontends for Treebeard
//!
//! This module contains implementations of the `LanguageFrontend` trait
//! for various languages that can target Treebeard.

pub mod rust;

#[cfg(feature = "oxur")]
pub mod oxur;

pub use rust::RustFrontend;

#[cfg(feature = "oxur")]
pub use oxur::OxurFrontend;
