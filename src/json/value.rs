use std::borrow::Cow;
use std::mem;

use crate::{
    de::{
        Deserialize,
        impls::{MapBuilder, VecBuilder},
        Map,
        Seq,
        VisitorSlot,
    },
    error::{Error, Result},
    json::{Array, Number, Object},
    private,
    Place,
    ser::{ValueView, Serialize},
};

/// Any valid JSON value.
///
/// This type has a non-recursive drop implementation so it is safe to build
/// arbitrarily deeply nested instances.
///
/// ```rust
/// use miniserde::json::{Array, Value};
///
/// let mut value = Value::Null;
/// for _ in 0..100000 {
///     let mut array = Array::new();
///     array.push(value);
///     value = Value::Array(array);
/// }
/// // no stack overflow when `value` goes out of scope
/// ```
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Array),
    Object(Object),
}

impl Default for Value {
    /// The default value is null.
    fn default() -> Self {
        Value::Null
    }
}

impl Serialize for Value {
    fn view(&self) -> ValueView {
        match self {
            Value::Null => ValueView::Null,
            Value::Bool(b) => ValueView::Bool(*b),
            Value::Number(Number::U64(n)) => ValueView::U64(*n),
            Value::Number(Number::I64(n)) => ValueView::I64(*n),
            Value::Number(Number::Float(n)) => ValueView::Float(*n),
            Value::String(s) => ValueView::Str(Cow::Borrowed(s)),
            Value::Array(array) => private::stream_slice(array),
            Value::Object(object) => private::stream_object(object),
        }
    }
}

impl Deserialize for Value {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl VisitorSlot for Place<Value> {
            fn write_null(&mut self) -> Result<()> {
                self.out = Some(Value::Null);
                Ok(())
            }

            fn write_boolean(&mut self, b: bool) -> Result<()> {
                self.out = Some(Value::Bool(b));
                Ok(())
            }

            fn write_string(&mut self, s: &str) -> Result<()> {
                self.out = Some(Value::String(s.to_owned()));
                Ok(())
            }

            fn write_integer(&mut self, i: i128) -> Result<()> {
                self.out = Some(Value::Number(if i > 0 {
                    if let Ok(n) = i.try_into() {
                        Number::U64(n)
                    } else {
                        return Err(Error);
                    }
                } else {
                    if let Ok(i) = i.try_into() {
                        Number::I64(i)
                    } else {
                        return Err(Error);
                    }
                }));
                Ok(())
            }

            fn write_float(&mut self, n: f64) -> Result<()> {
                self.out = Some(Value::Number(Number::Float(n)));
                Ok(())
            }

            fn with_seq_slots (
                self: &'_ mut Place<Value>,
                fill_seq: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn Seq>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut builder = VecBuilder {
                    vec: vec![],
                    next_slot: None,
                };
                let ret = fill_seq(&mut builder);
                builder.vec.extend(builder.next_slot);
                self.out = Some(Value::Array(builder.vec));
                Ok(ret)
            }

            fn with_map_slots (
                self: &'_ mut Place<Value>,
                with: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn Map>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut builder = MapBuilder {
                    map: Default::default(),
                    next_slot: (None, None),
                };
                let ret = with(&mut builder);
                if let (Some(k), Some(v)) = builder.next_slot {
                    builder.map.insert(k, v);
                }
                self.out = Some(Value::Object(builder.map));
                Ok(ret)
            }
        }

        Place::new(out)
    }
}
