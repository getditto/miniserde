//! Deserialization traits.
//!
//! Deserialization in miniserde works by returning a "place" into which data
//! may be written through the methods of the `VisitorSlot` trait object.
//!
//! Use the `make_place!` macro to acquire a "place" type. A library may use a
//! single place type across all of its Deserialize impls, or each impl or each
//! module may use a private place type. There is no difference.
//!
//! A place is simply:
//!
//! ```rust
//! struct Place<T> {
//!     out: Option<T>,
//! }
//! ```
//!
//! Upon successful deserialization the output object is written as `Some(T)`
//! into the `out` field of the place.
//!
//! ## Deserializing a primitive
//!
//! The VisitorSlot trait has a method corresponding to each supported primitive
//! type.
//!
//! ```rust
//! use miniserde::{make_place, Result};
//! use miniserde::de::{Deserialize, VisitorSlot};
//!
//! make_place!(Place);
//!
//! struct MyBoolean(bool);
//!
//! // The VisitorSlot trait has a selection of methods corresponding to different
//! // data types. We override the ones that our Rust type supports
//! // deserializing from, and write the result into the `out` field of our
//! // output place.
//! //
//! // These methods may perform validation and decide to return an error.
//! impl VisitorSlot for Place<MyBoolean> {
//!     fn boolean(&mut self, b: bool) -> Result<()> {
//!         self.out = Some(MyBoolean(b));
//!         Ok(())
//!     }
//! }
//!
//! impl Deserialize for MyBoolean {
//!     fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
//!         // All Deserialize impls will look exactly like this. There is no
//!         // other correct implementation of Deserialize.
//!         Place::new(out)
//!     }
//! }
//! ```
//!
//! ## Deserializing a sequence
//!
//! In the case of a sequence (JSON array), the VisitorSlot method returns a builder
//! that can hand out places to write sequence next_slots one next_slot at a time.
//!
//! ```rust
//! use miniserde::{make_place, Result};
//! use miniserde::de::{Deserialize, Seq, VisitorSlot};
//! use std::mem;
//!
//! make_place!(Place);
//!
//! struct MyVec<T>(Vec<T>);
//!
//! impl<T: Deserialize> VisitorSlot for Place<MyVec<T>> {
//!     fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
//!         Ok(Box::new(VecBuilder {
//!             out: &mut self.out,
//!             vec: Vec::new(),
//!             next_slot: None,
//!         }))
//!     }
//! }
//!
//! struct VecBuilder<'a, T: 'a> {
//!     // At the end, output will be written here.
//!     out: &'a mut Option<MyVec<T>>,
//!     // Previous next_slots are accumulated here.
//!     vec: Vec<T>,
//!     // Next next_slot will be placed here.
//!     next_slot: Option<T>,
//! }
//!
//! impl<'a, T: Deserialize> Seq for VecBuilder<'a, T> {
//!     fn next_slot(&mut self) -> Result<&mut dyn VisitorSlot> {
//!         // Free up the place by transfering the most recent next_slot
//!         // into self.vec.
//!         self.vec.extend(self.next_slot.take());
//!         // Hand out a place to write the next next_slot.
//!         Ok(Deserialize::begin(&mut self.next_slot))
//!     }
//!
//!     fn finish(&mut self) -> Result<()> {
//!         // Transfer the last next_slot.
//!         self.vec.extend(self.next_slot.take());
//!         // Move the output object into self.out.
//!         let vec = mem::replace(&mut self.vec, Vec::new());
//!         *self.out = Some(MyVec(vec));
//!         Ok(())
//!     }
//! }
//!
//! impl<T: Deserialize> Deserialize for MyVec<T> {
//!     fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
//!         // As mentioned, all Deserialize impls will look like this.
//!         Place::new(out)
//!     }
//! }
//! ```
//!
//! ## Deserializing a map or struct
//!
//! This code demonstrates what is generated for structs by
//! `#[derive(Deserialize)]`.
//!
//! ```rust
//! use miniserde::{make_place, Result};
//! use miniserde::de::{Deserialize, Map, VisitorSlot};
//!
//! make_place!(Place);
//!
//! // The struct that we would like to deserialize.
//! struct Demo {
//!     code: u32,
//!     message: String,
//! }
//!
//! impl VisitorSlot for Place<Demo> {
//!     fn map(&mut self) -> Result<Box<dyn Map + '_>> {
//!         // Like for sequences, we produce a builder that can hand out places
//!         // to write one struct field at a time.
//!         Ok(Box::new(DemoBuilder {
//!             code: None,
//!             message: None,
//!             out: &mut self.out,
//!         }))
//!     }
//! }
//!
//! struct DemoBuilder<'a> {
//!     code: Option<u32>,
//!     message: Option<String>,
//!     out: &'a mut Option<Demo>,
//! }
//!
//! impl<'a> Map for DemoBuilder<'a> {
//!     fn slot_at(&mut self, k: &str) -> Result<&mut dyn VisitorSlot> {
//!         // Figure out which field is being deserialized and return a place
//!         // to write it.
//!         //
//!         // The code here ignores unrecognized fields but an implementation
//!         // would be free to return an error instead. Similarly an
//!         // implementation may want to check for duplicate fields by
//!         // returning an error if the current field already has a value.
//!         match k {
//!             "code" => Ok(Deserialize::begin(&mut self.code)),
//!             "message" => Ok(Deserialize::begin(&mut self.message)),
//!             _ => Ok(VisitorSlot::ignore()),
//!         }
//!     }
//!
//!     fn finish(&mut self) -> Result<()> {
//!         // Make sure we have every field and then write the output object
//!         // into self.out.
//!         let code = self.code.take().ok_or(miniserde::Error)?;
//!         let message = self.message.take().ok_or(miniserde::Error)?;
//!         *self.out = Some(Demo { code, message });
//!         Ok(())
//!     }
//! }
//!
//! impl Deserialize for Demo {
//!     fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
//!         // All Deserialize impls look like this.
//!         Place::new(out)
//!     }
//! }
//! ```

use crate::error::{Error, Result};

pub(in crate)
mod impls;

pub(in crate)
type WithResult = Result<
    ::with_locals::dyn_safe::ContinuationReturn,
    ::with_locals::dyn_safe::ContinuationReturn,
>;

/// Trait for data structures that can be deserialized from a JSON string.
///
/// [Refer to the module documentation for examples.][::de]
pub trait Deserialize: Sized {
    /// The only correct implementation of this method is:
    ///
    /// ```rust
    /// # use miniserde::make_place;
    /// # use miniserde::de::{Deserialize, VisitorSlot};
    /// #
    /// # make_place!(Place);
    /// # struct S;
    /// # impl VisitorSlot for Place<S> {}
    /// #
    /// # impl Deserialize for S {
    /// fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
    ///     Place::new(out)
    /// }
    /// # }
    /// ```
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot;

    // Not public API. This method is only intended for Option<T>, should not
    // need to be implemented outside of this crate.
    #[doc(hidden)]
    #[inline]
    fn default() -> Option<Self> {
        None
    }
}

/// Trait that can write data into an output place.
///
/// [Refer to the module documentation for examples.][::de]
#[allow(unused)]
// #[with(dyn_safe = true)]
pub trait VisitorSlot {
    fn write_null(&mut self) -> Result<()> {
        Err(Error)
    }

    fn write_boolean(&mut self, b: bool) -> Result<()> {
        Err(Error)
    }

    fn write_string(&mut self, s: &str) -> Result<()> {
        Err(Error)
    }

    fn write_integer(&mut self, i: i128) -> Result<()> {
        Err(Error)
    }

    fn write_float(&mut self, x: f64) -> Result<()> {
        Err(Error)
    }

    fn with_seq_slots (
        self: &'_ mut Self,
        with: &'_ mut dyn (
            for<'local>
            FnMut(Result<&'local mut dyn Seq>)
              -> ::with_locals::dyn_safe::ContinuationReturn
        ),
    ) -> WithResult
    {
        Err(with(Err(Error)))
    }

    fn with_map_slots (
        self: &'_ mut Self,
        with: &'_ mut dyn (
            for<'local>
            FnMut(Result<&'local mut dyn Map>)
              -> ::with_locals::dyn_safe::ContinuationReturn
        ),
    ) -> WithResult
    {
        Err(with(Err(Error)))
    }
    // fn seq_slots(&mut self) -> Result<&'ref mut dyn Seq> {
    //     Err(Error)
    // }

    // fn map_slots(&mut self) -> Result<&'ref mut dyn Map> {
    //     Err(Error)
    // }
}

/// Trait that can hand out places to write sequence next_slots.
///
/// [Refer to the module documentation for examples.][crate::de]
pub trait Seq {
    fn next_slot(&mut self) -> Result<&mut dyn VisitorSlot>;
    // fn finish(&mut self) -> Result<()>;
}

/// Trait that can hand out places to write values of a map.
///
/// [Refer to the module documentation for examples.][crate::de]
pub trait Map {
    fn slot_at(&mut self, k: &str) -> Result<&mut dyn VisitorSlot>;
    // fn finish(&mut self) -> Result<()>;
}
