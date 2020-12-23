//! CBOR data format.
//!
//! [See the crate level doc](../index.html#example) for an example of
//! serializing and deserializing CBOR.

mod ser;
pub use self::ser::to_bytes;

mod de;
pub use self::de::from_bytes;

mod value;
pub use self::value::Value;

mod number;
pub use self::number::i65;

mod array;
pub use self::array::Array;

mod map;
pub use self::map::Map;

mod drop;
