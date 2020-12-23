//! Serialization traits.
//!
//! Serialization in miniserde works by traversing an input object and
//! decomposing it iteratively into a stream of fragments.
//!
//! ## Serializing a primitive
//!
//! ```rust
//! use miniserde::ser::{ValueView, Serialize};
//!
//! // The data structure that we want to serialize as a primitive.
//! struct MyBoolean(bool);
//!
//! impl Serialize for MyBoolean {
//!     fn view(&self) -> ValueView {
//!         ValueView::Bool(self.0)
//!     }
//! }
//! ```
//!
//! ## Serializing a sequence
//!
//! ```rust
//! use miniserde::ser::{ValueView, Seq, Serialize};
//!
//! // Some custom sequence type that we want to serialize.
//! struct MyVec<T>(Vec<T>);
//!
//! impl<T: Serialize> Serialize for MyVec<T> {
//!     fn view(&self) -> ValueView {
//!         ValueView::Seq(Box::new(SliceStream { iter: self.0.iter() }))
//!     }
//! }
//!
//! struct SliceStream<'a, T: 'a> {
//!     iter: std::slice::Iter<'a, T>,
//! }
//!
//! impl<'a, T: Serialize> Seq for SliceStream<'a, T> {
//!     fn next(&mut self) -> Option<&dyn Serialize> {
//!         let next_slot = self.iter.next()?;
//!         Some(next_slot)
//!     }
//! }
//! ```
//!
//! ## Serializing a map or struct
//!
//! This code demonstrates what is generated for structs by
//! `#[derive(Serialize)]`.
//!
//! ```rust
//! use miniserde::ser::{ValueView, Map, Serialize};
//! use std::borrow::Cow;
//!
//! // The struct that we would like to serialize.
//! struct Demo {
//!     code: u32,
//!     message: String,
//! }
//!
//! impl Serialize for Demo {
//!     fn view(&self) -> ValueView {
//!         ValueView::Map(Box::new(DemoStream {
//!             data: self,
//!             state: 0,
//!         }))
//!     }
//! }
//!
//! struct DemoStream<'a> {
//!     data: &'a Demo,
//!     state: usize,
//! }
//!
//! impl<'a> Map for DemoStream<'a> {
//!     fn next(&mut self) -> Option<(Cow<str>, &dyn Serialize)> {
//!         let state = self.state;
//!         self.state += 1;
//!         match state {
//!             0 => Some((Cow::Borrowed("code"), &self.data.code)),
//!             1 => Some((Cow::Borrowed("message"), &self.data.message)),
//!             _ => None,
//!         }
//!     }
//! }
//! ```

mod impls;

use std::borrow::Cow;

/// One unit of output produced during serialization.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub enum ValueView<'view> {
    Null,
    Bool(bool),
    Str(Cow<'view, str>),
    Int(i128),
    Float(f64),
    Seq(Box<dyn Seq<'view>>),
    Map(Box<dyn Map<'view>>),
}

/// Trait for data structures that can be serialized to a JSON string.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub trait Serialize {
    fn view(&self) -> ValueView<'_>;
}

/// Trait that can iterate next_slots of a sequence.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub trait Seq<'view> {
    fn next(&mut self) -> Option<&'view dyn Serialize>;
}

/// Trait that can iterate slot_at-value entries of a map or struct.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub trait Map<'view> {
    fn next(&mut self) -> Option<(Cow<'view, str>, &'view dyn Serialize)>;
}
