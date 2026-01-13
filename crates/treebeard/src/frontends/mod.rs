//! Language frontends for Treebeard
//!
//! This module contains implementations of the `LanguageFrontend` trait
//! for various languages that can target Treebeard.

pub mod rust;

pub use rust::RustFrontend;
