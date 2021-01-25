//! Deserialization traits.
//!
//! Deserialization in miniserde works by returning a "place" into which data
//! may be written through the methods of the `Visitor` trait object.
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
//! The Visitor trait has a method corresponding to each supported primitive
//! type.
//!
//! ```rust
//! use miniserde_ditto::{make_place, Result};
//! use miniserde_ditto::de::{Deserialize, Visitor};
//!
//! make_place!(Place);
//!
//! struct MyBoolean(bool);
//!
//! // The Visitor trait has a selection of methods corresponding to different
//! // data types. We override the ones that our Rust type supports
//! // deserializing from, and write the result into the `out` field of our
//! // output place.
//! //
//! // These methods may perform validation and decide to return an error.
//! impl Visitor for Place<MyBoolean> {
//!     fn boolean(&mut self, b: bool) -> Result<()> {
//!         self.out = Some(MyBoolean(b));
//!         Ok(())
//!     }
//! }
//!
//! impl Deserialize for MyBoolean {
//!     fn begin(out: &mut Option<Self>) -> &mut dyn Visitor {
//!         // All Deserialize impls will look exactly like this. There is no
//!         // other correct implementation of Deserialize.
//!         Place::new(out)
//!     }
//! }
//! ```
//!
//! ## Deserializing a sequence
//!
//! In the case of a sequence (JSON array), the visitor method returns a builder
//! that can hand out places to write sequence elements one element at a time.
//!
//! ```rust
//! use miniserde_ditto::{make_place, Result};
//! use miniserde_ditto::de::{Deserialize, Seq, Visitor};
//! use std::mem;
//!
//! make_place!(Place);
//!
//! struct MyVec<T>(Vec<T>);
//!
//! impl<T: Deserialize> Visitor for Place<MyVec<T>> {
//!     fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
//!         Ok(Box::new(VecBuilder {
//!             out: &mut self.out,
//!             vec: Vec::new(),
//!             element: None,
//!         }))
//!     }
//! }
//!
//! struct VecBuilder<'a, T: 'a> {
//!     // At the end, output will be written here.
//!     out: &'a mut Option<MyVec<T>>,
//!     // Previous elements are accumulated here.
//!     vec: Vec<T>,
//!     // Next element will be placed here.
//!     element: Option<T>,
//! }
//!
//! impl<'a, T: Deserialize> Seq for VecBuilder<'a, T> {
//!     fn element(&mut self) -> Result<&mut dyn Visitor> {
//!         // Free up the place by transfering the most recent element
//!         // into self.vec.
//!         self.vec.extend(self.element.take());
//!         // Hand out a place to write the next element.
//!         Ok(Deserialize::begin(&mut self.element))
//!     }
//!
//!     fn finish(self: Box<Self>) -> Result<()> {
//!         let mut vec = self.vec;
//!         // Transfer the last element.
//!         vec.extend(self.element);
//!         // Move the output object into self.out.
//!         *self.out = Some(MyVec(vec));
//!         Ok(())
//!     }
//! }
//!
//! impl<T: Deserialize> Deserialize for MyVec<T> {
//!     fn begin(out: &mut Option<Self>) -> &mut dyn Visitor {
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
//! use miniserde_ditto::{make_place, Result};
//! use miniserde_ditto::de::{Deserialize, Map, StrKeyMap, Visitor};
//!
//! make_place!(Place);
//!
//! // The struct that we would like to deserialize.
//! struct Demo {
//!     code: u32,
//!     message: String,
//! }
//!
//! impl Visitor for Place<Demo> {
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
//! impl<'a> StrKeyMap for DemoBuilder<'a> {
//!     fn key(&mut self, k: &str) -> Result<&mut dyn Visitor> {
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
//!             _ => Ok(Visitor::ignore()),
//!         }
//!     }
//!
//!     fn finish(self: Box<Self>) -> Result<()> {
//!         // Make sure we have every field and then write the output object
//!         // into self.out.
//!         let code = self.code.ok_or(miniserde_ditto::Error)?;
//!         let message = self.message.ok_or(miniserde_ditto::Error)?;
//!         *self.out = Some(Demo { code, message });
//!         Ok(())
//!     }
//! }
//!
//! impl Deserialize for Demo {
//!     fn begin(out: &mut Option<Self>) -> &mut dyn Visitor {
//!         // All Deserialize impls look like this.
//!         Place::new(out)
//!     }
//! }
//! ```

pub use ignored_any::IgnoredAny;
mod ignored_any;

mod impls;

use crate::Result;

use private::Private;
mod private {
    pub struct Private;
}

/// Trait for data structures that can be deserialized from a JSON string.
///
/// [Refer to the module documentation for examples.][crate::de]
pub trait Deserialize: Sized {
    /// The only correct implementation of this method is:
    ///
    /// ```rust
    /// # use miniserde_ditto::make_place;
    /// # use miniserde_ditto::de::{Deserialize, Visitor};
    /// #
    /// # make_place!(Place);
    /// # struct S;
    /// # impl Visitor for Place<S> {}
    /// #
    /// # impl Deserialize for S {
    /// fn begin(out: &mut Option<Self>) -> &mut dyn Visitor {
    ///     Place::new(out)
    /// }
    /// # }
    /// ```
    fn begin(out: &mut Option<Self>) -> &mut dyn Visitor;

    // Not public API. This method is only intended for Option<T>, should not
    // need to be implemented outside of this crate.
    #[doc(hidden)]
    #[inline]
    fn default() -> Option<Self> {
        None
    }

    // Specialization hacks to enable optimized deserialization into `u8` slices.
    // Not public API either, which is enforced thanks to the `Private`
    // parameter.
    #[doc(hidden)]
    #[::with_locals::with]
    fn bytes_visitor_vec(
        _: &'_ mut Vec<Self>,
        _: Private,
    ) -> Option<&'ref mut dyn FnMut(&'_ [u8])> {
        None
    }

    #[doc(hidden)]
    #[::with_locals::with]
    fn bytes_visitor_slice(
        _: &'_ mut [::core::mem::MaybeUninit<Self>],
        _: Private,
    ) -> Option<&'ref mut dyn FnMut(&'_ [u8]) -> Result<()>> {
        None
    }
}

/// Trait that can write data into an output place.
///
/// [Refer to the module documentation for examples.][crate::de]
#[allow(unused_variables)]
pub trait Visitor {
    fn null(&mut self) -> Result<()> {
        self.map()
            .and_then(|map| map.finish())
            .or_else(|_| err!("Failed to deserialize a `null` as an empty map at that position."))
    }

    fn boolean(&mut self, b: bool) -> Result<()> {
        err!(
            "Cannot deserialize a `boolean` (got {:?}) at that position.",
            b
        );
    }

    fn string(&mut self, s: &str) -> Result<()> {
        err!(
            "Cannot deserialize a `string` (got {:?}) at that position.",
            s
        );
    }

    fn bytes(&mut self, xs: &[u8]) -> Result<()> {
        self.seq()
            .and_then(|mut seq| {
                for &x in xs {
                    seq.element()?.int(x as _)?;
                }
                seq.finish()
            })
            .or_else(|_| {
                err!(
                    "Failed to deserialize a `bytes` (got {:#x?}) as a int-seq at that position.",
                    xs
                )
            })
    }

    fn int(&mut self, i: i128) -> Result<()> {
        err!("Cannot deserialize a `int` (got {:?}) at that position.", i);
    }

    fn float(&mut self, f: f64) -> Result<()> {
        err!(
            "Cannot deserialize a `float` (got {:?}) at that position.",
            f
        );
    }

    fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
        err!("Cannot deserialize a `seq` at that position.");
    }

    fn map(&mut self) -> Result<Box<dyn Map + '_>> {
        err!("Cannot deserialize a `map` at that position.");
    }
}

/// Trait that can hand out places to write sequence elements.
///
/// [Refer to the module documentation for examples.][crate::de]
pub trait Seq {
    fn element(&mut self) -> Result<&mut dyn Visitor>;
    fn finish(self: Box<Self>) -> Result<()>;
}

/// Trait that can hand out places to write values of a map.
///
/// In order to support arbitrary `impl Deserialize` keys, the API requires
/// yielding an out-slot for the key through a callback, and once this callback
/// returns, the implementor can inspect the just deserialized key to yield
/// the appropriate / matching out-slot value.
///
/// Since the signature is a bit complex, and most implementations only use
/// stringly-typed keys, **it is recommended to implement the much simpler
/// [`StrKeyMap`] convenience trait instead**.
///
/// [Refer to the module documentation for examples.][crate::de]
pub trait Map {
    fn val_with_key(
        &mut self,
        with_key: &mut dyn FnMut(Result<&mut dyn Visitor>) -> Result<()>,
    ) -> Result<&mut dyn Visitor>;
    fn finish(self: Box<Self>) -> Result<()>;
}

/// Convenience trait to automagically implement the more complex [`Map`] trait
/// in the case where only stringly-typed keys are to be deserialized (_e.g._,
/// when dealing with `struct`s).
pub trait StrKeyMap: Map {
    fn key(&mut self, k: &str) -> Result<&mut dyn Visitor>;

    fn finish(self: Box<Self>) -> Result<()>;
}

impl<T: StrKeyMap> Map for T {
    fn val_with_key(
        &mut self,
        de_key: &mut dyn FnMut(Result<&mut dyn Visitor>) -> Result<()>,
    ) -> Result<&mut dyn Visitor> {
        let mut s = None::<String>;
        de_key(Ok(Deserialize::begin(&mut s)))?;
        match s.as_deref() {
            Some(k) => self.key(k),
            None => err!("Encountered a non-string key when deserializing"),
        }
    }

    fn finish(self: Box<Self>) -> Result<()> {
        StrKeyMap::finish(self)
    }
}
