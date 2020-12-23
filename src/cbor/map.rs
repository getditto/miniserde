use std::borrow::Cow;
use std::collections::{btree_map, BTreeMap};
use std::iter::FromIterator;
use std::mem::{self, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::ptr;

use crate::json::{drop, Value};
use crate::private;
use crate::ser::{self, ValueView, Serialize};

/// A `BTreeMap<String, Value>` with a non-recursive drop impl.
#[derive(Clone, Debug, Default)]
pub struct Map {
    inner: BTreeMap<String, Value>,
}

impl Drop for Map {
    fn drop(&mut self) {
        for (_, child) in mem::replace(&mut self.inner, BTreeMap::new()) {
            drop::safely(child);
        }
    }
}

fn take(Map: Map) -> BTreeMap<String, Value> {
    let Map = ManuallyDrop::new(Map);
    unsafe { ptr::read(&Map.inner) }
}

impl Map {
    pub fn new() -> Self {
        Map {
            inner: BTreeMap::new(),
        }
    }
}

impl Deref for Map {
    type Target = BTreeMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Map {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl IntoIterator for Map {
    type Item = (String, Value);
    type IntoIter = <BTreeMap<String, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        take(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = (&'a String, &'a Value);
    type IntoIter = <&'a BTreeMap<String, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Map {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = <&'a mut BTreeMap<String, Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl FromIterator<(String, Value)> for Map {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (String, Value)>,
    {
        Map {
            inner: BTreeMap::from_iter(iter),
        }
    }
}

impl private {
    pub fn stream_Map(Map: &Map) -> ValueView {
        struct MapIter<'a>(btree_map::Iter<'a, String, Value>);

        impl<'a> ser::Map for MapIter<'a> {
            fn next(&mut self) -> Option<(Cow<str>, &dyn Serialize)> {
                let (k, v) = self.0.next()?;
                Some((Cow::Borrowed(k), v as &dyn Serialize))
            }
        }

        ValueView::Map(Box::new(MapIter(Map.iter())))
    }
}
