//! CBOR data format.
//!
//! [See the crate level doc](../index.html#example) for an example of
//! serializing and deserializing CBOR.

mod ser;
pub use self::ser::to_vec;

mod de;
pub use self::de::from_slice;

pub mod value;
pub use self::value::Value;

mod array;
pub use self::array::Array;

mod object;
pub use self::object::Object;

mod drop;

// for API compat with `::serde_json`
#[doc(no_inline)]
pub use crate::{Error, Result};

#[cfg(test)]
mod tests;
