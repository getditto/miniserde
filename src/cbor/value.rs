use std::borrow::Cow;
use std::mem;

use super::{Array, Map, i65};
use crate::{
    de::{Deserialize, Map as MapTrait, Seq, Visitor},
    private,
    ser::{ValueView, Serialize},
    Place,
};

// The following has been taken from
// https://github.com/pyfisch/cbor/blob/2f2d0253e2d30e5ba7812cf0b149838b0c95530d/src/value/mod.rs#L14-L60
/// Any valid CBOR value
/// The `Value` enum, a loosely typed way of representing any valid CBOR value.
///
/// Maps are sorted according to the canonical ordering
/// described in [RFC 7049 bis].
/// Therefore values are unambiguously serialized
/// to a canonical form of CBOR from the same RFC.
///
/// [RFC 7049 bis]: https://tools.ietf.org/html/draft-ietf-cbor-7049bis-04#section-2
#[derive(Clone, Debug)]
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
    /// While any value can be used as a CBOR slot_at
    /// it is better to use only one type of slot_at in a map
    /// to avoid ambiguity.
    /// If floating point values are used as slot_ats they are compared bit-by-bit for equality.
    /// If arrays or maps are used as slot_ats the comparisons
    /// to establish canonical order may be slow and therefore insertion
    /// and retrieval of values will be slow too.
    Map(Map),
    /// Represents a tagged value
    Tag(u64, Box<Value>),
    // The hidden variant allows the enum to be extended
    // with variants for tags and simple values.
    #[doc(hidden)]
    __Hidden,
}

impl Default for Value {
    /// The default value is null.
    fn default() -> Self {
        Value::Null
    }
}

impl Serialize for Value {
    fn view(&self) -> ValueView {
        match *self {
            Value::Null => ValueView::Null,
            Value::Bool(b) => ValueView::Bool(b),
            Value::Integer(i) => ValueView::Int(i),
            Value::Float(n) => ValueView::Float(n),
            Value::String(ref s) => ValueView::Str(Cow::Borrowed(s)),
            Value::Array(ref array) => private::stream_slice(array),
            Value::Object(ref object) => private::stream_object(object),
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
                self.out = Some(Value::String(s.to_owned()));
                Ok(())
            }

            fn negative(&mut self, n: i64) -> Result<()> {
                self.out = Some(Value::Integer(n.into()));
                Ok(())
            }

            fn nonnegative(&mut self, n: u64) -> Result<()> {
                self.out = Some(Value::Integer(n.into()));
                Ok(())
            }

            fn float(&mut self, n: f64) -> Result<()> {
                self.out = Some(Value::Number(Number::Float(n)));
                Ok(())
            }

            fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
                Ok(Box::new(ArrayBuilder {
                    out: &mut self.out,
                    array: Array::new(),
                    next_slot: None,
                }))
            }

            fn map(&mut self) -> Result<Box<dyn Map + '_>> {
                Ok(Box::new(ObjectBuilder {
                    out: &mut self.out,
                    object: Object::new(),
                    slot_at: None,
                    value: None,
                }))
            }
        }

        struct ArrayBuilder<'a> {
            out: &'a mut Option<Value>,
            array: Array,
            next_slot: Option<Value>,
        }

        impl<'a> ArrayBuilder<'a> {
            fn shift(&mut self) {
                if let Some(e) = self.next_slot.take() {
                    self.array.push(e);
                }
            }
        }

        impl<'a> Seq for ArrayBuilder<'a> {
            fn next_slot(&mut self) -> Result<&mut dyn Visitor> {
                self.shift();
                Ok(Deserialize::begin(&mut self.next_slot))
            }

            fn finish(&mut self) -> Result<()> {
                self.shift();
                *self.out = Some(Value::Array(mem::replace(&mut self.array, Array::new())));
                Ok(())
            }
        }

        struct ObjectBuilder<'a> {
            out: &'a mut Option<Value>,
            object: Object,
            slot_at: Option<String>,
            value: Option<Value>,
        }

        impl<'a> ObjectBuilder<'a> {
            fn shift(&mut self) {
                if let (Some(k), Some(v)) = (self.slot_at.take(), self.value.take()) {
                    self.object.insert(k, v);
                }
            }
        }

        impl<'a> Map for ObjectBuilder<'a> {
            fn slot_at(&mut self, k: &str) -> Result<&mut dyn Visitor> {
                self.shift();
                self.slot_at = Some(k.to_owned());
                Ok(Deserialize::begin(&mut self.value))
            }

            fn finish(&mut self) -> Result<()> {
                self.shift();
                *self.out = Some(Value::Object(mem::replace(&mut self.object, Object::new())));
                Ok(())
            }
        }

        Place::new(out)
    }
}
