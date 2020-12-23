use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};
use std::mem;
use std::str::FromStr;

use crate::{
    Place,
    de::{self, Deserialize, VisitorSlot},
    error::{Error, Result},
};

impl Deserialize for () {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl VisitorSlot for Place<()> {
            fn write_null(&mut self) -> Result<()> {
                self.out = Some(());
                Ok(())
            }
        }
        Place::new(out)
    }
}

impl Deserialize for bool {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl VisitorSlot for Place<bool> {
            fn write_boolean(&mut self, b: bool) -> Result<()> {
                self.out = Some(b);
                Ok(())
            }
        }
        Place::new(out)
    }
}

impl Deserialize for String {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl VisitorSlot for Place<String> {
            fn write_string(&mut self, s: &str) -> Result<()> {
                self.out = Some(s.to_owned());
                Ok(())
            }
        }
        Place::new(out)
    }
}

impl_Deserialize_for_ints! {
    i8, i16, i32, isize, i64, i128,
    u8, u16, u32, usize, u64,
} macro_rules! impl_Deserialize_for_ints {(
    $($xN:ident ,)*
) => (
    $(
        impl Deserialize for $xN {
            fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
                impl VisitorSlot for Place<$xN> {
                    fn write_integer(&mut self, i: i128) -> Result<()> {
                        if let Ok(x) = ::core::convert::TryInto::try_into(i) {
                            self.out = Some(x);
                            Ok(())
                        } else {
                            Err(Error)
                        }
                    }
                }
                Place::new(out)
            }
        }
    )*
)} use impl_Deserialize_for_ints;

macro_rules! impl_Deserialize_for_float {(
    $fN:ident
) => (
    impl Deserialize for $fN {
        fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
            impl VisitorSlot for Place<$fN> {
                fn write_integer(&mut self, i: i128) -> Result<()> {
                    self.out = Some(i as $fN);
                    Ok(())
                }

                fn write_float(&mut self, n: f64) -> Result<()> {
                    self.out = Some(n as $fN);
                    Ok(())
                }
            }
            Place::new(out)
        }
    }
)}
impl_Deserialize_for_float!(f32);
impl_Deserialize_for_float!(f64);

impl<T: Deserialize> Deserialize for Box<T> {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl<T: Deserialize> VisitorSlot for Place<Box<T>> {
            fn write_null(&mut self) -> Result<()> {
                let mut out = None;
                Deserialize::begin(&mut out).write_null()?;
                self.out = Some(Box::new(out.unwrap()));
                Ok(())
            }

            fn write_boolean(&mut self, b: bool) -> Result<()> {
                let mut out = None;
                Deserialize::begin(&mut out).write_boolean(b)?;
                self.out = Some(Box::new(out.unwrap()));
                Ok(())
            }

            fn write_string(&mut self, s: &str) -> Result<()> {
                let mut out = None;
                Deserialize::begin(&mut out).write_string(s)?;
                self.out = Some(Box::new(out.unwrap()));
                Ok(())
            }

            fn write_integer(&mut self, i: i128) -> Result<()> {
                let mut out = None;
                Deserialize::begin(&mut out).write_integer(i)?;
                self.out = Some(Box::new(out.unwrap()));
                Ok(())
            }

            fn write_float(&mut self, n: f64) -> Result<()> {
                let mut out = None;
                Deserialize::begin(&mut out).write_float(n)?;
                self.out = Some(Box::new(out.unwrap()));
                Ok(())
            }

            fn with_seq_slots (
                self: &'_ mut Self,
                fill_seq: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Seq>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut stack_slot = None::<T>;
                let ret =
                    <T as Deserialize>::begin(&mut stack_slot)
                        .with_seq_slots(fill_seq)?
                ;
                (if let Some(value) = stack_slot {
                    self.out = Some(Box::new(value));
                    Ok
                } else {
                    Err
                })(ret)
            }

            fn with_map_slots (
                self: &'_ mut Place<Box<T>>,
                fill_seq: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Map>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut stack_slot = None::<T>;
                let ret =
                    <T as Deserialize>::begin(&mut stack_slot)
                        .with_map_slots(fill_seq)?
                ;
                self.out = Some(Box::new(stack_slot.unwrap()));
                Ok(ret)
            }
        }

        Place::new(out)
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    #[inline]
    fn default() -> Option<Self> {
        Some(None)
    }
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl<T: Deserialize> VisitorSlot for Place<Option<T>> {
            fn write_null(&mut self) -> Result<()> {
                self.out = Some(None);
                Ok(())
            }

            fn write_boolean(&mut self, b: bool) -> Result<()> {
                self.out = Some(None);
                Deserialize::begin(self.out.as_mut().unwrap()).write_boolean(b)
            }

            fn write_string(&mut self, s: &str) -> Result<()> {
                self.out = Some(None);
                Deserialize::begin(self.out.as_mut().unwrap()).write_string(s)
            }

            fn write_integer(&mut self, i: i128) -> Result<()> {
                self.out = Some(None);
                Deserialize::begin(self.out.as_mut().unwrap()).write_integer(i)
            }

            fn write_float(&mut self, n: f64) -> Result<()> {
                self.out = Some(None);
                Deserialize::begin(self.out.as_mut().unwrap()).write_float(n)
            }

            fn with_seq_slots (
                self: &'_ mut Self,
                with: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Seq>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                self.out = Some(None);
                Deserialize::begin(self.out.as_mut().unwrap())
                    .with_seq_slots(with)
            }

            fn with_map_slots (
                self: &'_ mut Self,
                with: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Map>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                self.out = Some(None);
                Deserialize::begin(self.out.as_mut().unwrap())
                    .with_map_slots(with)
            }
        }

        Place::new(out)
    }
}

impl<A: Deserialize, B: Deserialize> Deserialize for (A, B) {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        // #[with(dyn_safe = true)]
        impl<A: Deserialize, B: Deserialize> VisitorSlot for Place<(A, B)> {
            fn with_seq_slots (
                self: &'_ mut Place<(A, B)>,
                with: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Seq>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                struct TupleBuilder<A, B> (
                    Option<A>,
                    Option<B>,
                );
                impl<A: Deserialize, B: Deserialize> de::Seq
                    for TupleBuilder<A, B>
                {
                    fn next_slot(&mut self) -> Result<&mut dyn VisitorSlot> {
                        if self.0.is_none() {
                            Ok(Deserialize::begin(&mut self.0))
                        } else if self.1.is_none() {
                            Ok(Deserialize::begin(&mut self.1))
                        } else {
                            Err(Error)
                        }
                    }
                }
                let mut tuple = TupleBuilder(None, None);
                let ret = with(Ok(&mut tuple));
                (match tuple {
                    | TupleBuilder(Some(a), Some(b)) => {
                        self.out = Some((a, b));
                        Ok
                    },
                    | _ => Err,
                })(ret)
            }
        }

        Place::new(out)
    }
}

pub(in crate)
struct VecBuilder<T> {
    pub(in crate)
    vec: Vec<T>,

    pub(in crate)
    next_slot: Option<T>,
}

impl<T: Deserialize> de::Seq for VecBuilder<T> {
    fn next_slot (self: &'_ mut Self)
      -> Result<&'_ mut dyn VisitorSlot>
    {
        self.vec.extend(self.next_slot.take());
        Ok(Deserialize::begin(&mut self.next_slot))
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl<T: Deserialize> VisitorSlot for Place<Vec<T>> {
            fn with_seq_slots (
                self: &'_ mut Place<Vec<T>>,
                fill_seq: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Seq>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut builder = VecBuilder {
                    vec: vec![],
                    next_slot: None,
                };
                let ret = fill_seq(Ok(&mut builder));
                builder.vec.extend(builder.next_slot);
                self.out = Some(builder.vec);
                Ok(ret)
            }
        }

        Place::new(out)
    }
}

pub(in crate)
struct MapBuilder<Map, K, V> {
    pub(in crate)
    map: Map,

    pub(in crate)
    next_slot: (Option<K>, Option<V>),
}

impl<Map, K, V> de::Map for MapBuilder<Map, K, V>
where
    Map : Extend<(K, V)>,
    K: FromStr,
    V : Deserialize,
{
    fn slot_at(&mut self, k: &str) -> Result<&mut dyn VisitorSlot> {
        if let (Some(k), Some(v)) = mem::take(&mut self.next_slot) {
            self.map.extend(Some((k, v)));
        }
        self.next_slot.0 = Some(match K::from_str(k) {
            Ok(slot_at) => slot_at,
            Err(_) => return Err(Error),
        });
        Ok(Deserialize::begin(&mut self.next_slot.1))
    }
}

impl<K, V, H> Deserialize for HashMap<K, V, H>
where
    K: FromStr + Hash + Eq,
    V: Deserialize,
    H: BuildHasher + Default,
{
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl<K, V, H> VisitorSlot for Place<HashMap<K, V, H>>
        where
            K: FromStr + Hash + Eq,
            V: Deserialize,
            H: BuildHasher + Default,
        {
            fn with_map_slots (
                self: &'_ mut Self,
                with: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Map>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut builder = MapBuilder {
                    map: HashMap::with_hasher(H::default()),
                    next_slot: (None, None),
                };
                let ret = with(Ok(&mut builder));
                if let (Some(k), Some(v)) = builder.next_slot {
                    builder.map.insert(k, v);
                }
                self.out = Some(builder.map);
                Ok(ret)
            }
        }

        Place::new(out)
    }
}

impl<K: FromStr + Ord, V: Deserialize> Deserialize for BTreeMap<K, V> {
    fn begin(out: &mut Option<Self>) -> &mut dyn VisitorSlot {
        impl<K: FromStr + Ord, V: Deserialize> VisitorSlot for Place<BTreeMap<K, V>> {
            fn with_map_slots (
                self: &'_ mut Self,
                with: &'_ mut dyn (
                    for<'local>
                    FnMut(Result<&'local mut dyn de::Map>)
                      -> ::with_locals::dyn_safe::ContinuationReturn
                ),
            ) -> crate::de::WithResult
            {
                let mut builder = MapBuilder {
                    map: BTreeMap::new(),
                    next_slot: (None, None),
                };
                let ret = with(Ok(&mut builder));
                if let (Some(k), Some(v)) = builder.next_slot {
                    builder.map.insert(k, v);
                }
                self.out = Some(builder.map);
                Ok(ret)
            }
        }

        Place::new(out)
    }
}
