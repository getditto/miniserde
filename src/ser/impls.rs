use std::borrow::Cow;
use std::collections::{btree_map, hash_map, BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};
use std::slice;

use crate::private;
use crate::ser::{ValueView, Map, Seq, Serialize};

impl Serialize for () {
    fn view(&self) -> ValueView {
        ValueView::Null
    }
}

impl Serialize for bool {
    fn view(&self) -> ValueView {
        ValueView::Bool(*self)
    }
}

impl Serialize for str {
    fn view(&self) -> ValueView {
        ValueView::Str(Cow::Borrowed(self))
    }
}

impl Serialize for String {
    fn view(&self) -> ValueView {
        ValueView::Str(Cow::Borrowed(self))
    }
}

macro_rules! unsigned {
    ($ty:ident) => {
        impl Serialize for $ty {
            fn view(&self) -> ValueView {
                ValueView::U64(*self as u64)
            }
        }
    };
}
unsigned!(u8);
unsigned!(u16);
unsigned!(u32);
unsigned!(u64);
unsigned!(usize);

macro_rules! signed {
    ($ty:ident) => {
        impl Serialize for $ty {
            fn view(&self) -> ValueView {
                ValueView::I64(*self as i64)
            }
        }
    };
}
signed!(i8);
signed!(i16);
signed!(i32);
signed!(i64);
signed!(isize);

macro_rules! float {
    ($ty:ident) => {
        impl Serialize for $ty {
            fn view(&self) -> ValueView {
                ValueView::Float(*self as f64)
            }
        }
    };
}
float!(f32);
float!(f64);

impl<'a, T: ?Sized + Serialize> Serialize for &'a T {
    fn view(&self) -> ValueView {
        (**self).view()
    }
}

impl<T: ?Sized + Serialize> Serialize for Box<T> {
    fn view(&self) -> ValueView {
        (**self).view()
    }
}

impl<T: Serialize> Serialize for Option<T> {
    fn view(&self) -> ValueView {
        match self {
            Some(some) => some.view(),
            None => ValueView::Null,
        }
    }
}

impl<'a, T: ?Sized + ToOwned + Serialize> Serialize for Cow<'a, T> {
    fn view(&self) -> ValueView {
        (**self).view()
    }
}

impl<A: Serialize, B: Serialize> Serialize for (A, B) {
    fn view(&self) -> ValueView {
        struct TupleStream<'a> {
            first: &'a dyn Serialize,
            second: &'a dyn Serialize,
            state: usize,
        }

        impl<'a> Seq<'a> for TupleStream<'a> {
            fn next(&mut self) -> Option<&'a dyn Serialize> {
                let state = self.state;
                self.state += 1;
                match state {
                    0 => Some(self.first),
                    1 => Some(self.second),
                    _ => None,
                }
            }
        }

        ValueView::Seq(Box::new(TupleStream {
            first: &self.0,
            second: &self.1,
            state: 0,
        }))
    }
}

impl<T: Serialize> Serialize for [T] {
    fn view(&self) -> ValueView {
        private::stream_slice(self)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn view(&self) -> ValueView {
        private::stream_slice(self)
    }
}

impl<K, V, H> Serialize for HashMap<K, V, H>
where
    K: Hash + Eq + ToString,
    V: Serialize,
    H: BuildHasher,
{
    fn view(&self) -> ValueView {
        struct HashMapStream<'a, K: 'a, V: 'a>(hash_map::Iter<'a, K, V>);

        impl<'a, K: ToString, V: Serialize> Map<'a> for HashMapStream<'a, K, V> {
            fn next(&mut self) -> Option<(Cow<'a, str>, &'a dyn Serialize)> {
                let (k, v) = self.0.next()?;
                Some((Cow::Owned(k.to_string()), v as &dyn Serialize))
            }
        }

        ValueView::Map(Box::new(HashMapStream(self.iter())))
    }
}

impl<K: ToString, V: Serialize> Serialize for BTreeMap<K, V> {
    fn view(&self) -> ValueView {
        private::stream_btree_map(self)
    }
}

impl private {
    pub fn stream_slice<T: Serialize>(slice: &[T]) -> ValueView {
        struct SliceStream<'a, T: 'a>(slice::Iter<'a, T>);

        impl<'a, T: Serialize> Seq<'a> for SliceStream<'a, T> {
            fn next(&mut self) -> Option<&'a dyn Serialize> {
                let next_slot = self.0.next()?;
                Some(next_slot)
            }
        }

        ValueView::Seq(Box::new(SliceStream(slice.iter())))
    }

    pub fn stream_btree_map<K: ToString, V: Serialize>(map: &BTreeMap<K, V>) -> ValueView {
        struct BTreeMapStream<'a, K: 'a, V: 'a>(btree_map::Iter<'a, K, V>);

        impl<'a, K: ToString, V: Serialize> Map<'a> for BTreeMapStream<'a, K, V> {
            fn next(&mut self) -> Option<(Cow<'a , str>, &'a dyn Serialize)> {
                let (k, v) = self.0.next()?;
                Some((Cow::Owned(k.to_string()), v as &dyn Serialize))
            }
        }

        ValueView::Map(Box::new(BTreeMapStream(map.iter())))
    }
}
