use std::borrow::Cow;

use crate::de::{Deserialize, Map, Seq, Visitor};
use crate::error::Result;
use crate::json::{Array, Number, Object};
use crate::private;
use crate::ser::{Serialize, ValueView};
use crate::Place;

/// Any valid JSON value.
///
/// This type has a non-recursive drop implementation so it is safe to build
/// arbitrarily deeply nested instances.
///
/// ```rust
/// use miniserde_ditto::json::{Array, Value};
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
    fn begin(&self) -> ValueView<'_> {
        match self {
            Value::Null => ValueView::Null,
            Value::Bool(b) => ValueView::Bool(*b),
            &Value::Number(Number::U64(n)) => ValueView::Int(n as _),
            &Value::Number(Number::I64(i)) => ValueView::Int(i as _),
            &Value::Number(Number::F64(f)) => ValueView::F64(f),
            Value::String(s) => ValueView::Str(Cow::Borrowed(s)),
            Value::Array(array) => private::stream_slice(array),
            Value::Object(object) => private::stream_json_object(object),
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

            fn int(&mut self, i: i128) -> Result<()> {
                use ::core::convert::TryFrom;
                self.out = Some(Value::Number(if let Ok(u64) = u64::try_from(i) {
                    Number::U64(u64)
                } else if let Ok(i64) = i64::try_from(i) {
                    Number::I64(i64)
                } else {
                    return Err(crate::Error);
                }));
                Ok(())
            }

            fn float(&mut self, n: f64) -> Result<()> {
                self.out = Some(Value::Number(Number::F64(n)));
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
            key: Option<String>,
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
                *self.out = Some(Value::Object(self.object));
                Ok(())
            }
        }

        Place::new(out)
    }
}
