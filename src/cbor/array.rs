use std::iter::FromIterator;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr;

use super::{drop, Value};

/// A `Vec<Value>` with a non-recursive drop impl.
#[derive(Clone, Debug, Default)]
pub struct Array(pub Vec<Value>);

impl Drop for Array {
    fn drop(&mut self) {
        self.0.drain(..).for_each(drop::safely);
    }
}

fn take(array: Array) -> Vec<Value> {
    let array = ManuallyDrop::new(array);
    unsafe { ptr::read(&array.0) }
}

impl Array {
    pub fn new() -> Self {
        Array { 0: Vec::new() }
    }
}

impl Deref for Array {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Array {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = <Vec<Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        take(self).into_iter()
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = <&'a Vec<Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut Array {
    type Item = &'a mut Value;
    type IntoIter = <&'a mut Vec<Value> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl FromIterator<Value> for Array {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Value>,
    {
        Array {
            0: Vec::from_iter(iter),
        }
    }
}
