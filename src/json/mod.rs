//! JSON data format.
//!
//! [See the crate level doc](../index.html#example) for an example of
//! serializing and deserializing JSON.

mod ser;
pub use self::ser::to_string;

mod de;
pub use self::de::from_str;

mod value;
pub use self::value::Value;

mod number;
pub use self::number::Number;

mod array;
pub use self::array::Array;

mod object;
pub use self::object::Object;

pub fn to_value<T: crate::Serialize>(v: T) -> crate::Result<Value> {
    // Inefficient polyfill implementation.
    from_str(&to_string(&v)?)
}

pub fn from_value<T: crate::Deserialize>(v: Value) -> crate::Result<T> {
    // Inefficient polyfill implementation.
    from_str(&to_string(&v)?)
}

// for API compat with `::serde_json`
#[doc(no_inline)]
pub use crate::{Error, Result};

mod drop;
