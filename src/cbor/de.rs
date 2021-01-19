use std::borrow::Cow;

use ::core::convert::TryFrom;

use crate::de::{Deserialize, Visitor};
use crate::error::{Error, Result};

/// Deserialize a CBOR byte sequence into any deserializable type.
///
/// ```rust
/// use miniserde_ditto::{cbor, Deserialize};
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Example {
///     code: u32,
///     message: String,
/// }
///
/// fn main() -> miniserde_ditto::Result<()> {
///     let j = &[
///         0xa2, // 2-long map
///
///             0x64, // 4-long str
///                 b'c', b'o', b'd', b'e',
///             0x18, // positive u8 > 24.
///                 0xc8, // 200 = 0xc8
///
///             0x67, // 7-long str
///                 b'm', b'e', b's', b's', b'a', b'g', b'e',
///             0x74, // str of length: 0x14 = 20.
///                 b'R', b'e', b'm', b'i', b'n', b'i', b's', b'c', b'e', b'n', b't',
///                 b' ', b'o', b'f', b' ',
///                 b'S', b'e', b'r', b'd', b'e',
///     ][..];
///     let out: Example = cbor::from_slice(&j)?;
///     println!("{:?}", out);
///
///     assert_eq!(out, Example { code: 200, message: "Reminiscent of Serde".into() });
///     Ok(())
/// }
/// ```
pub fn from_slice<T: Deserialize>(bytes: &[u8]) -> Result<T> {
    let mut out = None;
    let ref mut cursor = bytes.iter();
    from_slice_impl(cursor, T::begin(&mut out))
        .and_then(|()| {
            if cursor.as_slice().is_empty() {
                out
            } else {
                err!(
                    "Trailing bytes in CBOR deserialization. Remaining = {:#x?}",
                    cursor.as_slice()
                );
            }
        })
        .ok_or(Error)
}

const MAX_DEPTH: u16 = 256;

fn from_slice_impl<'bytes>(
    bytes: &'_ mut ::core::slice::Iter<'bytes, u8>,
    visitor: &mut dyn Visitor,
) -> Option<()> {
    use helpers::*;

    // Avoid accidental unchecked recursion; use a thread local to track depth:
    let from_slice_impl = ();
    drop(from_slice_impl);
    fn recurse_checked<'bytes>(
        bytes: &'_ mut ::core::slice::Iter<'bytes, u8>,
        visitor: &mut dyn Visitor,
    ) -> Option<()> {
        thread_local! {
            static CUR_DEPTH: ::core::cell::Cell<u16> = 0.into();
        }
        let ret = if CUR_DEPTH.with(|it| it.replace(it.get() + 1)) > MAX_DEPTH {
            err!("Reached maximum depth / recursion when deserializing CBOR object.");
        } else {
            self::from_slice_impl(bytes, visitor)
        };
        CUR_DEPTH.with(|it| it.set(it.get() - 1));
        ret
    }

    match major_and_tag(bytes.next()?) {
        (m @ major::INT!(), tag) => {
            let mut value: i128 = parse_u64(tag, bytes)? as _;
            if m == major::NEG_INT {
                value = -(value + 1);
            }
            visitor.int(value).ok()?;
        }

        (major::BYTE_SLICE, tag::UNKNOWN_LEN) => {
            let ref mut acc_bytes: Cow<'bytes, [u8]> = vec![].into();
            loop {
                match major_and_tag(bytes.next()?) {
                    BREAK_CODE => break,
                    (major::BYTE_SLICE, tag) => {
                        let chunk = parse_known_len_byte_seq(tag, bytes)?;
                        if acc_bytes.is_empty() {
                            *acc_bytes = chunk.into();
                        } else {
                            acc_bytes.to_mut().extend_from_slice(chunk);
                        }
                    }
                    _ => err!(
                        r#"Expected \xff or a known-len byte slice. Remaining = {:#x?}"#,
                        bytes.as_slice(),
                    ),
                }
            }
            visitor.bytes(acc_bytes).ok()?;
        }
        (major::BYTE_SLICE, tag) => {
            let slice = parse_known_len_byte_seq(tag, bytes)?;
            visitor.bytes(slice).ok()?;
        }

        (major::STR, tag::UNKNOWN_LEN) => {
            let ref mut acc_str: Cow<'bytes, str> = String::new().into();
            loop {
                match major_and_tag(bytes.next()?) {
                    BREAK_CODE => break,
                    (major::BYTE_SLICE, tag) => {
                        let chunk = parse_known_len_byte_seq(tag, bytes)?;
                        let s = ::core::str::from_utf8(chunk).ok()?;
                        if acc_str.is_empty() {
                            *acc_str = s.into();
                        } else {
                            acc_str.to_mut().push_str(s);
                        }
                    }
                    _ => err!(
                        r#"Expected \xff or a known-len string. Remaining = {:#x?}"#,
                        bytes.as_slice(),
                    ),
                }
            }
            visitor.string(acc_str).ok()?;
        }
        (major::STR, tag) => {
            let slice = parse_known_len_byte_seq(tag, bytes)?;
            let s = ::core::str::from_utf8(slice).ok()?;
            visitor.string(s).ok()?;
        }

        (major::SEQ, tag::UNKNOWN_LEN) => {
            let mut seq = visitor.seq().ok()?;
            loop {
                if major_and_tag(bytes.as_slice().get(0)?) == BREAK_CODE {
                    break;
                }
                recurse_checked(bytes, seq.element().ok()?)?;
            }
            seq.finish().ok()?;
        }
        (major::SEQ, tag) => {
            let len = usize::try_from(parse_u64(tag, bytes)?).ok()?;
            let mut seq = visitor.seq().ok()?;
            for _ in 0..len {
                recurse_checked(bytes, seq.element().ok()?)?;
            }
            seq.finish().ok()?;
        }
        (major::MAP, tag::UNKNOWN_LEN) => {
            let mut map = visitor.map().ok()?;
            loop {
                if major_and_tag(bytes.as_slice().get(0)?) == BREAK_CODE {
                    break;
                }

                let out_v = map
                    .val_with_key(&mut |it| {
                        it.and_then(|out_k| recurse_checked(bytes, out_k).ok_or(crate::Error))
                    })
                    .ok()?;
                recurse_checked(bytes, out_v)?;
            }
            map.finish().ok()?;
        }
        (major::MAP, tag) => {
            let len = usize::try_from(parse_u64(tag, bytes)?).ok()?;
            let mut map = visitor.map().ok()?;
            for _ in 0..len {
                let out_v = map
                    .val_with_key(&mut |it| {
                        it.and_then(|out_k| recurse_checked(bytes, out_k).ok_or(crate::Error))
                    })
                    .ok()?;
                recurse_checked(bytes, out_v)?;
            }
            map.finish().ok()?;
        }

        (major::CUSTOM_TAG, tag) => err!("Custom tag (tag = {:#x}) cannot be deserialized", tag),

        (major::FLOAT_BOOL_OR_UNIT, t @ tag::bool::TRUE)
        | (major::FLOAT_BOOL_OR_UNIT, t @ tag::bool::FALSE) => {
            visitor.boolean(t == tag::bool::TRUE).ok()?;
        }

        (major::FLOAT_BOOL_OR_UNIT, tag::UNIT_CANONICAL)
        | (major::FLOAT_BOOL_OR_UNIT, tag::UNIT_ALTERNATIVE) => {
            visitor.null().ok()?;
        }

        (major::FLOAT_BOOL_OR_UNIT, t @ tag::FLOAT!()) => {
            use ::half::f16;
            let f: f64 = match t {
                tag::FLOAT::_16 => {
                    f16::from_bits(u16::from_be_bytes(multi_bytes!(bytes, 2))).into()
                }
                tag::FLOAT::_32 => {
                    f32::from_bits(u32::from_be_bytes(multi_bytes!(bytes, 4))).into()
                }
                tag::FLOAT::_64 => {
                    f64::from_bits(u64::from_be_bytes(multi_bytes!(bytes, 8))).into()
                }
                _ => unreachable!(),
            };
            visitor.float(f).ok()?;
        }

        (major::FLOAT_BOOL_OR_UNIT, _) => err!(
            r#"Incorrect tag associated to major 7. Remaining = {:#x?}"#,
            bytes.as_slice(),
        ),

        _ => unreachable!(),
    }
    Some(())
}

mod helpers {
    use super::*;

    pub fn major_and_tag(&byte: &'_ u8) -> (u8, u8) {
        (byte >> 5, byte & 0x1f)
    }

    #[rustfmt::skip]
    pub mod major {
        pub const POS_INT: u8 = 0;
        pub const NEG_INT: u8 = 1;
        macro_rules! INT {() => (
            major::POS_INT ..= major::NEG_INT
        )} pub(in crate) use INT;
        pub const BYTE_SLICE: u8 = 2;
        pub const STR: u8 = 3;
        pub const SEQ: u8 = 4;
        pub const MAP: u8 = 5;
        pub const CUSTOM_TAG: u8 = 6;
        pub const FLOAT_BOOL_OR_UNIT: u8 = 7;
    }

    #[rustfmt::skip]
    pub mod tag {
        pub const SMALL_U8_MAX: u8 = 0x17;
        pub const U8 : u8 = 0x18;
        pub const U16: u8 = 0x19;
        pub const U32: u8 = 0x1a;
        pub const U64: u8 = 0x1b;
        pub const UNKNOWN_LEN: u8 = 0x1f;
        pub mod bool {
            pub const FALSE: u8 = 0x14;
            pub const TRUE: u8 = 0x15;
        }
        pub const UNIT_CANONICAL: u8 = 0x16;
        pub const UNIT_ALTERNATIVE: u8 = 0x17;
        #[allow(nonstandard_style)]
        pub(in crate) mod FLOAT {
            pub const _16: u8 = 0x19;
            pub const _32: u8 = 0x1a;
            pub const _64: u8 = 0x1b;
        }
        macro_rules! FLOAT_ {() => (
            tag::FLOAT::_16 ..= tag::FLOAT::_64
        )} pub(in crate) use FLOAT_ as FLOAT;
    }

    pub const BREAK_CODE: (u8, u8) = (
        // major
        7,
        // tag
        tag::UNKNOWN_LEN,
    );

    macro_rules! multi_bytes {($bytes:expr, $N:expr) => ({
        use ::uninit::prelude::*;
        let mut buf = uninit_array![u8; $N];
        <[u8; $N] as ::core::convert::TryFrom<_>>::try_from(
            buf .as_out()
                .init_with(::core::iter::from_fn(|| {
                    $bytes.next().map(|&b| b)
                }))
                .as_ref()
        ).ok().or_else(|| err!("Expected {} bytes to deserialize an integer", $N))?
    })}
    pub(in crate) use multi_bytes;

    pub fn parse_u64(tag: u8, bytes: &'_ mut ::core::slice::Iter<'_, u8>) -> Option<u64> {
        Some({
            match tag {
                small_u8 @ 0..=tag::SMALL_U8_MAX => small_u8 as _,
                tag::U8 => *bytes.next()? as _,
                tag::U16 => u16::from_be_bytes(multi_bytes!(bytes, 2)) as _,
                tag::U32 => u32::from_be_bytes(multi_bytes!(bytes, 4)) as _,
                tag::U64 => u64::from_be_bytes(multi_bytes!(bytes, 8)) as _,
                _ => err!(
                    "Incorrect integer tag. Remaining = {:#x?}",
                    bytes.as_slice()
                ),
            }
        })
    }

    pub fn parse_known_len_byte_seq<'input>(
        tag: u8,
        bytes: &'_ mut ::core::slice::Iter<'input, u8>,
    ) -> Option<&'input [u8]> {
        let len = usize::try_from(parse_u64(tag, bytes)?).ok()?;
        let slice = bytes.as_slice();
        *bytes = slice.get(len..)?.iter();
        Some(&slice[..len])
    }
}
