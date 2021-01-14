use std::borrow::Cow;
use std::collections::{btree_map, BTreeMap};
use std::iter::FromIterator;
use std::mem::{self, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::ptr;

use super::{drop, Value};
use crate::private;
use crate::ser::{self, Serialize, ValueView};

/// A `BTreeMap<Value, Value>` with a non-recursive drop impl.
#[derive(Clone, Debug, Default)]
pub struct Object {
    inner: BTreeMap<Value, Value>,
}

impl Drop for Object {
    fn drop(&mut self) {
        for (key, child) in mem::replace(&mut self.inner, BTreeMap::new()) {
            drop::safely(key);
            drop::safely(child);
        }
    }
}

fn take(object: Object) -> BTreeMap<Value, Value> {
    let object = ManuallyDrop::new(object);
    unsafe { ptr::read(&object.inner) }
}

impl Object {
    pub fn new() -> Self {
        Object {
            inner: BTreeMap::new(),
        }
    }
}

impl Deref for Object {
    type Target = BTreeMap<Value, Value>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Object {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl IntoIterator for Object {
    type Item = (Value, Value);
    type IntoIter = <BTreeMap<Value, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        take(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = (&'a Value, &'a Value);
    type IntoIter = <&'a BTreeMap<Value, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Object {
    type Item = (&'a Value, &'a mut Value);
    type IntoIter = <&'a mut BTreeMap<Value, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl FromIterator<(Value, Value)> for Object {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (Value, Value)>,
    {
        Object {
            inner: BTreeMap::from_iter(iter),
        }
    }
}

impl private {
    pub fn stream_cbor_object(object: &Object) -> ValueView<'_> {
        struct ObjectIter<'a>(btree_map::Iter<'a, Value, Value>);

        impl<'a> ser::Map<'a> for ObjectIter<'a> {
            fn next(&mut self) -> Option<(Cow<'a, [u8]>, &'a dyn Serialize)> {
                let (k, v) = self.0.next()?;
                Some((Cow::Owned(super::to_vec(k).unwrap()), v as &dyn Serialize))
            }

            fn remaining(&self) -> usize {
                self.0.len()
            }
        }

        ValueView::Map(Box::new(ObjectIter(object.iter())))
    }
}
