use ::std::{borrow::Cow, cmp::Ordering};

use super::{Array, Object};
use crate::de::{Deserialize, Map, Seq, Visitor};
use crate::error::Result;
use crate::private;
use crate::ser::{Serialize, ValueView};
use crate::Place;

// Taken (and maybe modified) https://github.com/pyfisch/cbor/blob/2f2d0253e2d30e5ba7812cf0b149838b0c95530d/src/value/mod.rs
/// The `Value` enum, a loosely typed way of representing any valid CBOR value.
///
/// Maps are sorted according to the canonical ordering
/// described in [RFC 7049 bis].
/// Therefore values are unambiguously serialized
/// to a canonical form of CBOR from the same RFC.
///
/// [RFC 7049 bis]: https://tools.ietf.org/html/draft-ietf-cbor-7049bis-04#section-2
#[derive(Clone, Debug)]
#[non_exhaustive] // This allows the enum to be extended with variants for tags and simple values.
pub enum Value {
    /// Represents the absence of a value or the value undefined.
    Null,
    /// Represents a boolean value.
    Bool(bool),
    /// Integer CBOR numbers.
    ///
    /// The biggest value that can be represented is 2^64 - 1.
    /// While the smallest value is -2^64.
    /// Values outside this range can't be serialized
    /// and will cause an error.
    Integer(i128),
    /// Represents a floating point value.
    Float(f64),
    /// Represents a byte string.
    Bytes(Vec<u8>),
    /// Represents an UTF-8 encoded string.
    Text(String),
    /// Represents an array of values.
    Array(Array),
    /// Represents a map.
    ///
    /// Maps are also called tables, dictionaries, hashes, or objects (in JSON).
    /// While any value can be used as a CBOR key
    /// it is better to use only one type of key in a map
    /// to avoid ambiguity.
    /// If floating point values are used as keys they are compared bit-by-bit for equality.
    /// If arrays or maps are used as keys the comparisons
    /// to establish canonical order may be slow and therefore insertion
    /// and retrieval of values will be slow too.
    Map(Object),
    /// Represents a tagged value
    Tag(u64, Box<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Value) -> Ordering {
        // Determine the canonical order of two values:
        // 1. Smaller major type sorts first.
        // 2. Shorter sequence sorts first.
        // 3. Compare integers by magnitude.
        // 4. Compare byte and text sequences lexically.
        // 5. Compare the serializations of both types. (expensive)
        use self::Value::*;
        if self.major_type() != other.major_type() {
            return self.major_type().cmp(&other.major_type());
        }
        match (self, other) {
            (Integer(a), Integer(b)) => a.abs().cmp(&b.abs()),
            (Bytes(a), Bytes(b)) if a.len() != b.len() => a.len().cmp(&b.len()),
            (Text(a), Text(b)) if a.len() != b.len() => a.len().cmp(&b.len()),
            (Array(a), Array(b)) if a.len() != b.len() => a.len().cmp(&b.len()),
            (Map(a), Map(b)) if a.len() != b.len() => a.len().cmp(&b.len()),
            (Bytes(a), Bytes(b)) => a.cmp(b),
            (Text(a), Text(b)) => a.cmp(b),
            (a, b) => {
                let a = super::to_vec(a).expect("self is serializable");
                let b = super::to_vec(b).expect("other is serializable");
                a.cmp(&b)
            }
        }
    }
}

impl Default for Value {
    /// The default value is null.
    fn default() -> Self {
        Value::Null
    }
}

impl Serialize for Value {
    fn view(&self) -> ValueView<'_> {
        match self {
            Value::Null => ValueView::Null,
            Value::Bool(b) => ValueView::Bool(*b),
            &Value::Integer(i) => ValueView::Int(i),
            &Value::Float(f) => ValueView::F64(f),
            Value::Bytes(bytes) => private::stream_slice(bytes),
            Value::Text(s) => ValueView::Str(Cow::Borrowed(s)),
            Value::Array(array) => private::stream_slice(array),
            Value::Map(map) => private::stream_cbor_object(map),
            Value::Tag(..) => unimplemented!("Serializing tags is not supported by this crate."),
        }
    }
}

impl Deserialize for Value {
    fn begin(out: &mut Option<Self>) -> &mut dyn Visitor {
        impl Visitor for Place<Value> {
            fn null(&mut self) -> Result<()> {
                self.out = Some(Value::Null);
                Ok(())
            }

            fn boolean(&mut self, b: bool) -> Result<()> {
                self.out = Some(Value::Bool(b));
                Ok(())
            }

            fn string(&mut self, s: &str) -> Result<()> {
                self.out = Some(Value::Text(s.to_owned()));
                Ok(())
            }

            fn int(&mut self, i: i128) -> Result<()> {
                const MIN: i128 = -(1_i128 << 64);
                const MAX: i128 = (1_i128 << 64) - 1;
                match i {
                    MIN..=MAX => {
                        self.out = Some(Value::Integer(i));
                        Ok(())
                    }
                    _ => err!("Integer out of CBOR range"),
                }
            }

            fn float(&mut self, f: f64) -> Result<()> {
                self.out = Some(Value::Float(f));
                Ok(())
            }

            fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
                Ok(Box::new(ArrayBuilder {
                    out: &mut self.out,
                    array: Array::new(),
                    element: None,
                }))
            }

            fn map(&mut self) -> Result<Box<dyn Map + '_>> {
                Ok(Box::new(ObjectBuilder {
                    out: &mut self.out,
                    object: Object::new(),
                    key: None,
                    value: None,
                }))
            }
        }

        struct ArrayBuilder<'a> {
            out: &'a mut Option<Value>,
            array: Array,
            element: Option<Value>,
        }

        impl<'a> ArrayBuilder<'a> {
            fn shift(&mut self) {
                if let Some(e) = self.element.take() {
                    self.array.push(e);
                }
            }
        }

        impl<'a> Seq for ArrayBuilder<'a> {
            fn element(&mut self) -> Result<&mut dyn Visitor> {
                self.shift();
                Ok(Deserialize::begin(&mut self.element))
            }

            fn finish(mut self: Box<Self>) -> Result<()> {
                self.shift();
                *self.out = Some(Value::Array(self.array));
                Ok(())
            }
        }

        struct ObjectBuilder<'a> {
            out: &'a mut Option<Value>,
            object: Object,
            key: Option<Value>,
            value: Option<Value>,
        }

        impl<'a> ObjectBuilder<'a> {
            fn shift(&mut self) {
                if let (Some(k), Some(v)) = (self.key.take(), self.value.take()) {
                    self.object.insert(k, v);
                }
            }
        }

        impl<'a> Map for ObjectBuilder<'a> {
            fn val_with_key(
                &mut self,
                de_key: &mut dyn FnMut(Result<&mut dyn Visitor>) -> Result<()>,
            ) -> Result<&mut dyn Visitor> {
                self.shift();
                de_key(Ok(Deserialize::begin(&mut self.key)))?;
                Ok(Deserialize::begin(&mut self.value))
            }

            fn finish(mut self: Box<Self>) -> Result<()> {
                self.shift();
                *self.out = Some(Value::Map(self.object));
                Ok(())
            }
        }

        Place::new(out)
    }
}

impl Value {
    fn major_type(&self) -> u8 {
        use self::Value::*;
        match self {
            Null => 7,
            Bool(_) => 7,
            Integer(v) => {
                if *v >= 0 {
                    0
                } else {
                    1
                }
            }
            Tag(_, _) => 6,
            Float(_) => 7,
            Bytes(_) => 2,
            Text(_) => 3,
            Array(_) => 4,
            Map(_) => 5,
        }
    }
}

impl_From! {
    bool => Bool,
    i8 => Integer,
    i16 => Integer,
    i32 => Integer,
    i64 => Integer,
    // i128 omitted because not all numbers fit in CBOR serialization
    u8 => Integer,
    u16 => Integer,
    u32 => Integer,
    u64 => Integer,
    // u128 omitted because not all numbers fit in CBOR serialization
    f32 => Float,
    f64 => Float,
    // TODO: figure out if these impls should be more generic or removed.
    Vec<u8> => Bytes,
    String => Text,
}
/// where:
macro_rules! impl_From {(
    $(
        $T:ty => $Variant:ident
    ),* $(,)?
) => (
    $(
        impl From<$T> for Value {
            fn from (it: $T)
              -> Value
            {
                Value::$Variant(it.into())
            }
        }
    )*
)}
use impl_From;
