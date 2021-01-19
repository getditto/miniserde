#![allow(unused)]#![warn(unused_must_use)]

use crate::{
    ser::{Map, Seq, Serialize, ValueView},
    Result,
};
use ::std::io::{self, Write as _};

/// Serialize any serializable type into a CBOR byte sequence.
///
/// ```rust
/// use miniserde_ditto::{json, Serialize};
///
/// #[derive(Serialize, Debug)]
/// struct Example {
///     code: u32,
///     message: String,
/// }
///
/// fn main() {
///     let example = Example {
///         code: 200,
///         message: "reminiscent of Serde".to_owned(),
///     };
///
///     let j = json::to_string(&example).unwrap();
///     println!("{}", j);
/// }
/// ```
pub fn to_vec<T: ?Sized + Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut v = vec![];
    match to_writer(&value, &mut v) {
        Ok(()) => Ok(v),
        Err(None) => Err(crate::Error),
        Err(Some(io_err)) => unreachable!("IO failure on a Vec: {}", io_err),
    }
}

struct Serializer<'a> {
    stack: Vec<Layer<'a>>,
}

enum Layer<'a> {
    Seq(Box<dyn Seq<'a> + 'a>),
    Map(Box<dyn Map<'a> + 'a>),
}

impl<'a> Drop for Serializer<'a> {
    fn drop(&mut self) {
        // Drop layers in reverse order.
        while !self.stack.is_empty() {
            self.stack.pop();
        }
    }
}

#[allow(nonstandard_style)]
struct write_u64 {
    major: u8,
    v: u64,
}

impl write_u64 {
    fn into(self, out: &'_ mut (dyn io::Write)) -> io::Result<()> {
        let Self { major, v: value } = self;
        let mask = major << 5;
        macro_rules! with_uNs {( $($uN:ident)<* ) => ({
            mod c {
            $(
                pub mod $uN { pub const MAX: u64 = ::core::$uN::MAX as _; }
            )*
                pub mod u8 { pub const MAX: u64 = ::core::u8::MAX as _; }
            }
            const SMALL_U8_MAX: u64 = 0x17;
            #[allow(nonstandard_style)]
            enum MaskFor {
                u8 = (SMALL_U8_MAX + 1) as _,
                $($uN),*
            }
            match value {
                0 ..= SMALL_U8_MAX => out.write_all(&[mask | (value as u8)]),
                0 ..= c::u8::MAX => out.write_all(&[
                    mask | (MaskFor::u8 as u8),
                    value as u8,
                ]),
            $(
                0 ..= c::$uN::MAX => {
                    let value = value as $uN;
                    let ref mut buf = [0; 1 + ::core::mem::size_of::<$uN>()];
                    buf[0] = mask | (MaskFor::$uN as u8);
                    buf[1 ..].copy_from_slice(&value.to_be_bytes());
                    out.write_all(buf)
                },
            )*
                _ => unreachable!(),
            }
        })}
        with_uNs!(u16 < u32 < u64)
    }
}

/// Serialize any serializable type as a CBOR byte sequence into a
/// [`Write`][io::Write]able sink.
///
/// Returns:
///   - `Ok(())` on success.
///   - `Err(Some(io_error))` on I/O failure.
///   - `Err(None)` on serialization error (unrepresentable integer).
pub fn to_writer<'value>(
    value: &'value dyn Serialize,
    out: &'_ mut dyn io::Write,
) -> Result<(), Option<io::Error>> {
    // Borrow-checker-friendly "closure"
    #[cfg_attr(rustfmt, rustfmt::skip)]
    macro_rules! write { ($bytes:expr) => ({
        out.write_all($bytes).map_err(Some)
    })}

    // Use a manual stack to avoid (stack-allocated) recursion.
    let mut stack: Vec<Layer<'value>> = vec![Layer::Single(value)];
    // where:
    enum Layer<'value> {
        Seq(Box<dyn Seq<'value> + 'value>),
        Map(Box<dyn Map<'value> + 'value>),
        Single(&'value dyn Serialize),
    }
    while let Some(last) = stack.last_mut() {
        let view: ValueView<'value> = match last {
            &mut Layer::Single(value) => {
                let view = value.view();
                drop(stack.pop());
                view
            }
            Layer::Seq(seq) => {
                match seq.next() {
                    Some(value) => stack.push(Layer::Single(value)),
                    None => drop(stack.pop()),
                }
                continue;
            }
            Layer::Map(map) => {
                match map.next() {
                    Some((key, value)) => {
                        stack.push(Layer::Single(value));
                        stack.push(Layer::Single(key));
                    }
                    None => drop(stack.pop()),
                }
                continue;
            }
        };
        match view {
            ValueView::Null => write!(&[0xf6])?,
            ValueView::Bool(b) => write!(&[0xf4 | (b as u8)])?,
            ValueView::Str(s) => {
                write_u64 {
                    major: 3,
                    v: s.len() as u64,
                }
                .into(out)?;
                write!(s.as_bytes())?;
            }
            ValueView::Bytes(bs) => {
                write_u64 {
                    major: 2,
                    v: bs.len() as u64,
                }
                .into(out)?;
                write!(&*bs)?;
            }
            ValueView::Int(i) => {
                const MIN: i128 = -(1_i128 << 64);
                const MAX: i128 = ::core::u64::MAX as _;
                match i {
                    MIN..=-1 => write_u64 {
                        major: 1,
                        v: (-(i + 1)) as u64,
                    }
                    .into(out)?,
                    0..=MAX => write_u64 {
                        major: 0,
                        v: i as u64,
                    }
                    .into(out)?,
                    _ => err!("Cannot serialize integer {:?} as CBOR: out of range", i),
                }
            }
            ValueView::F64(f) if f.is_infinite() => write!(if f.is_sign_positive() {
                &[0xf9, 0x7c, 0x00]
            } else {
                &[0xf9, 0xfc, 0x00]
            })?,
            ValueView::F64(f) if f.is_nan() => {
                write!(&[0xf9, 0x7e, 0x00])?;
            }
            ValueView::F64(f) => {
                // Finite float.
                let f_16;
                let f_32;
                match () {
                    _case
                        if {
                            f_16 = ::half::f16::from_f64(f);
                            f64::from(f_16) == f
                        } =>
                    {
                        let ref mut buf = [0xf9, 0, 0];
                        buf[1..].copy_from_slice(&f_16.to_bits().to_be_bytes());
                        write!(buf)?;
                    }
                    _case
                        if {
                            f_32 = f as f32;
                            f64::from(f_32) == f
                        } =>
                    {
                        let ref mut buf = [0xfa, 0, 0, 0, 0];
                        buf[1..].copy_from_slice(&f_32.to_bits().to_be_bytes());
                        write!(buf)?;
                    }
                    _default => {
                        let ref mut buf = [0xfb, 0, 0, 0, 0, 0, 0, 0, 0];
                        buf[1..].copy_from_slice(&f.to_bits().to_be_bytes());
                        write!(buf)?;
                    }
                }
            }
            ValueView::Seq(mut seq) => {
                let count = seq.remaining();
                write_u64 {
                    major: 4,
                    v: count as _,
                }
                .into(out)?;
                stack.push(Layer::Seq(seq));
            }
            ValueView::Map(mut map) => {
                let count = map.remaining();
                write_u64 {
                    major: 5,
                    v: count as _,
                }
                .into(out)?;
                stack.push(Layer::Map(map));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{cbor::*, Serialize};

    macro_rules! assert_eq_hex {
        (
        $left:expr,
        $right:expr $(,)?
    ) => {
            match (&$left[..], &$right[..]) {
                (ref left, ref right) => {
                    if <[u8] as ::core::cmp::PartialEq>::ne(left, right) {
                        panic!(
                            "assertion failed: (`{}` == `{}`)\n{}]",
                            stringify!($left),
                            stringify!($right),
                            (0..left.len().max(right.len()))
                                .map(|i| match (left.get(i), right.get(i)) {
                                    (Some(l), Some(r)) => format!(
                                        "  {:01}|{:02x} – {:01}|{:02x},\n",
                                        l >> 5,
                                        l & 0x1f,
                                        r >> 5,
                                        r & 0x1f
                                    ),
                                    (Some(l), _) =>
                                        format!("  {:01}|{:02x} - ____,\n", l >> 5, l & 0x1f),
                                    (_, Some(r)) =>
                                        format!("____ - {:01}|{:02x},\n", r >> 5, r & 0x1f),
                                    _ => unreachable!(),
                                })
                                .collect::<String>(),
                        );
                    }
                }
            }
        };
    }

    #[test]
    fn test_str() {
        serialize_and_compare("foobar", b"ffoobar");
    }

    #[test]
    fn test_list() {
        serialize_and_compare(&[1, 2, 3][..], b"\x83\x01\x02\x03");
    }

    #[test]
    fn test_float() {
        serialize_and_compare(12.3f64, b"\xfb@(\x99\x99\x99\x99\x99\x9a");
    }

    #[test]
    fn test_integer() {
        // u8
        serialize_and_compare(24, b"\x18\x18");
        // i8
        serialize_and_compare(-5, b"\x24");
        // i16
        serialize_and_compare(-300, b"\x39\x01\x2b");
        // i32
        serialize_and_compare(-23567997, b"\x3a\x01\x67\x9e\x7c");
        // u64
        serialize_and_compare(::core::u64::MAX, b"\x1b\xff\xff\xff\xff\xff\xff\xff\xff");
    }

    fn serialize_and_compare<T: Serialize>(value: T, expected: &[u8]) {
        assert_eq_hex!(&to_vec(&value).unwrap()[..], expected,);
    }

    mod std {
        use super::*;
        use ::std::collections::BTreeMap;

        #[test]
        fn test_string() {
            let value = "foobar".to_owned();
            assert_eq_hex!(&to_vec(&value).unwrap()[..], b"ffoobar");
        }

        #[test]
        fn test_list() {
            let value = vec![1, 2, 3];
            assert_eq_hex!(&to_vec(&value).unwrap()[..], b"\x83\x01\x02\x03");
        }

        #[test]
        fn test_list_strings() {
            let value = vec!["1", "2", "3"];
            assert_eq_hex!(&to_vec(&value).unwrap()[..], b"\x83\x611\x612\x613");
        }

        #[test]
        fn test_object() {
            use ::std::collections::HashMap;
            let mut object = HashMap::new();
            object.insert("a".to_owned(), "A".to_owned());
            object.insert("b".to_owned(), "B".to_owned());
            object.insert("c".to_owned(), "C".to_owned());
            object.insert("d".to_owned(), "D".to_owned());
            object.insert("e".to_owned(), "E".to_owned());
            let vec = to_vec(&object).unwrap();
            let test_object = from_slice(&vec[..]).unwrap();
            assert_eq!(object, test_object);
        }

        #[test]
        fn test_object_list_keys() {
            let mut object = BTreeMap::new();
            object.insert(vec![0i64], ());
            object.insert(vec![100i64], ());
            object.insert(vec![-1i64], ());
            object.insert(vec![-2i64], ());
            object.insert(vec![0i64, 0i64], ());
            object.insert(vec![0i64, -1i64], ());
            let vec = to_vec(&to_value(&object).unwrap()).unwrap();
            assert_eq_hex!(
                vec![
                    166, 129, 0, 246, 129, 24, 100, 246, 129, 32, 246, 129, 33, 246, 130, 0, 0,
                    246, 130, 0, 32, 246
                ],
                vec
            );
            let test_object = from_slice(&vec[..]).unwrap();
            assert_eq!(object, test_object);
        }

        #[test]
        fn test_object_object_keys() {
            use ::std::iter::FromIterator;
            let mut object = BTreeMap::new();
            let keys = vec![
                vec!["a"],
                vec!["b"],
                vec!["c"],
                vec!["d"],
                vec!["aa"],
                vec!["a", "aa"],
            ]
            .into_iter()
            .map(|v| BTreeMap::from_iter(v.into_iter().map(|s| (s.to_owned(), ()))));

            for key in keys {
                object.insert(key, ());
            }
            let vec = to_vec(&to_value(&object).unwrap()).unwrap();
            assert_eq_hex!(
                vec![
                    166, 161, 97, 97, 246, 246, 161, 97, 98, 246, 246, 161, 97, 99, 246, 246, 161,
                    97, 100, 246, 246, 161, 98, 97, 97, 246, 246, 162, 97, 97, 246, 98, 97, 97,
                    246, 246
                ],
                vec
            );
            let test_object = from_slice(&vec[..]).unwrap();
            assert_eq!(object, test_object);
        }

        #[test]
        fn test_float() {
            let vec = to_vec(&12.3f64).unwrap();
            assert_eq_hex!(vec, b"\xfb@(\x99\x99\x99\x99\x99\x9a");
        }

        #[test]
        fn test_f32() {
            let vec = to_vec(&4000.5f32).unwrap();
            assert_eq_hex!(vec, b"\xfa\x45\x7a\x08\x00");
        }

        #[test]
        fn test_infinity() {
            let vec = to_vec(&::std::f64::INFINITY).unwrap();
            assert_eq_hex!(vec, b"\xf9|\x00");
        }

        #[test]
        fn test_neg_infinity() {
            let vec = to_vec(&::std::f64::NEG_INFINITY).unwrap();
            assert_eq_hex!(vec, b"\xf9\xfc\x00");
        }

        #[test]
        fn test_nan() {
            let vec = to_vec(&::std::f32::NAN).unwrap();
            assert_eq_hex!(vec, b"\xf9\x7e\x00");
        }

        #[test]
        fn test_integer() {
            // u8
            let vec = to_vec(&24).unwrap();
            assert_eq_hex!(vec, b"\x18\x18");
            // i8
            let vec = to_vec(&-5).unwrap();
            assert_eq_hex!(vec, b"\x24");
            // i16
            let vec = to_vec(&-300).unwrap();
            assert_eq_hex!(vec, b"\x39\x01\x2b");
            // i32
            let vec = to_vec(&-23567997).unwrap();
            assert_eq_hex!(vec, b"\x3a\x01\x67\x9e\x7c");
            // u64
            let vec = to_vec(&::std::u64::MAX).unwrap();
            assert_eq_hex!(vec, b"\x1b\xff\xff\xff\xff\xff\xff\xff\xff");
        }

        // #[test]
        // fn test_self_describing() {
        //     let mut vec = Vec::new();
        //     {
        //         let mut serializer = ser::Serializer::new(&mut vec);
        //         serializer.self_describe().unwrap();
        //         serializer.serialize_u64(9).unwrap();
        //     }
        //     assert_eq_hex!(vec, b"\xd9\xd9\xf7\x09");
        // }

        // #[test]
        // fn test_ip_addr() {
        //     use ::std::net::Ipv4Addr;

        //     let addr = Ipv4Addr::new(8, 8, 8, 8);
        //     let vec = to_vec(&addr).unwrap();
        //     println!("{:?}", vec);
        //     assert_eq_hex!(vec.len(), 5);
        //     let test_addr: Ipv4Addr = from_slice(&vec).unwrap();
        //     assert_eq_hex!(addr, test_addr);
        // }

        /// Test all of CBOR's fixed-length byte string types
        #[test]
        fn test_byte_string() {
            // Very short byte strings have 1-byte headers
            let short = vec![0_u8, 1, 2, 255];
            let short_s = to_vec(&short).unwrap();
            assert_eq_hex!(&short_s[..], [0x44, 0, 1, 2, 255]);

            // byte strings > 23 bytes have 2-byte headers
            let medium = vec![
                0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                255,
            ];
            let medium_s = to_vec(&medium).unwrap();
            assert_eq_hex!(
                &medium_s[..],
                [
                    0x58, 24, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
                    20, 21, 22, 255
                ]
            );

            // byte strings > 256 bytes have 3-byte headers
            let long_vec = (0..256).map(|i| (i & 0xFF) as u8).collect::<Vec<_>>();
            let long_s = to_vec(&long_vec).unwrap();
            assert_eq_hex!(&long_s[0..3], [0x59, 1, 0]);
            assert_eq_hex!(&long_s[3..], &long_vec[..]);

            // byte strings > 2^16 bytes have 5-byte headers
            let very_long_vec = (0..65536).map(|i| (i & 0xFF) as u8).collect::<Vec<_>>();
            let very_long_s = to_vec(&very_long_vec).unwrap();
            assert_eq_hex!(&very_long_s[0..5], [0x5a, 0, 1, 0, 0]);
            assert_eq_hex!(&very_long_s[5..], &very_long_vec[..]);

            // byte strings > 2^32 bytes have 9-byte headers, but they take too much RAM
            // to test in Travis.
        }

        #[test]
        fn test_half() {
            let vec = to_vec(&42.5f32).unwrap();
            assert_eq_hex!(vec, b"\xF9\x51\x50");
            assert_eq!(from_slice::<f32>(&vec[..]).unwrap(), 42.5f32);
        }
    }
}
