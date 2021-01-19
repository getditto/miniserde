//! Serialization traits.
//!
//! Serialization in miniserde works by traversing an input object and
//! decomposing it iteratively into a stream of fragments.
//!
//! ## Serializing a primitive
//!
//! ```rust
//! use miniserde_ditto::ser::{ValueView, Serialize};
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
//! use miniserde_ditto::ser::{ValueView, Seq, Serialize};
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
//! struct SliceStream<'view, T: 'view> {
//!     iter: std::slice::Iter<'view, T>,
//! }
//!
//! impl<'view, T: Serialize> Seq<'view> for SliceStream<'view, T> {
//!     fn next(&mut self) -> Option<&'view dyn Serialize> {
//!         let element = self.iter.next()?;
//!         Some(element)
//!     }
//!
//!     fn remaining(&self) -> usize {
//!         self.iter.as_slice().len()
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
//! use miniserde_ditto::ser::{ValueView, Map, Serialize};
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
//! struct DemoStream<'view> {
//!     data: &'view Demo,
//!     state: usize,
//! }
//!
//! impl<'view> Map<'view> for DemoStream<'view> {
//!     fn next(&mut self) -> Option<(&'view dyn Serialize, &'view dyn Serialize)> {
//!         let state = self.state;
//!         self.state += 1;
//!         match state {
//!             0 => Some((&"code", &self.data.code)),
//!             1 => Some((&"message", &self.data.message)),
//!             _ => None,
//!         }
//!     }
//!     fn remaining(&self) -> usize {
//!         2 - self.state
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
    Bytes(Cow<'view, [u8]>),
    Int(i128),
    F64(f64),
    Seq(Box<dyn Seq<'view> + 'view>),
    Map(Box<dyn Map<'view> + 'view>),
}

#[cfg(any())] // uncomment when debugging.
impl ::core::fmt::Debug for ValueView<'_> {
    fn fmt(self: &'_ Self, fmt: &'_ mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        use ValueView::*;
        match *self {
            Null => fmt.write_str("Null"),
            Bool(ref b) => fmt.debug_tuple("Bool").field(b).finish(),
            Str(ref s) => fmt.debug_tuple("Str").field(s).finish(),
            Bytes(ref xs) => fmt.debug_tuple("Str").field(xs).finish(),
            Int(ref i) => fmt.debug_tuple("Int").field(i).finish(),
            F64(ref f) => fmt.debug_tuple("F64").field(f).finish(),
            Seq(ref seq) => fmt
                .debug_struct("Seq")
                .field("remaining", &seq.remaining())
                .finish(),
            Map(ref map) => fmt
                .debug_struct("Map")
                .field("remaining", &map.remaining())
                .finish(),
        }
    }
}

impl ValueView<'_> {
    // Used by the JSON format when serializing keys
    pub(in crate) fn as_str(&self) -> Option<&'_ str> {
        match *self {
            ValueView::Bytes(ref xs) => Some(::core::str::from_utf8(xs).ok()?),
            ValueView::Str(ref s) => Some(s),
            _ => None,
        }
    }
}

/// Trait for data structures that can be serialized to a JSON string.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub trait Serialize {
    fn view(&self) -> ValueView<'_>;

    fn view_seq(seq: &'_ [Self]) -> ValueView<'_>
    where
        Self: Sized,
    {
        crate::private::stream_slice(seq)
    }
}

/// Trait that can iterate elements of a sequence.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub trait Seq<'view> {
    fn next(&mut self) -> Option<&'view dyn Serialize>;
    fn remaining(&self) -> usize;
}

/// Trait that can iterate key-value entries of a map or struct.
///
/// [Refer to the module documentation for examples.][crate::ser]
pub trait Map<'view> {
    fn next(&mut self) -> Option<(&'view dyn Serialize, &'view dyn Serialize)>;
    fn remaining(&self) -> usize;
}
