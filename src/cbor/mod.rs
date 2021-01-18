//! JSON data format.
//!
//! [See the crate level doc](../index.html#example) for an example of
//! serializing and deserializing JSON.

mod ser;
pub use self::ser::to_vec;

mod de;
pub use self::de::from_slice;

mod value;
pub use self::value::Value;

mod array;
pub use self::array::Array;

mod object;
pub use self::object::Object;

pub fn to_value<T: crate::Serialize>(v: T) -> crate::Result<Value> {
    from_slice(&to_vec(&v)?)
}

mod drop;

#[cfg(test)]
mod tests;
