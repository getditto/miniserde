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
///     let j = json::to_string(&example);
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
    #[rustfmt::skip]
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
                let view = value.begin();
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
                    Some((ref key_bytes, value)) => {
                        write!(key_bytes)?;
                        stack.push(Layer::Single(value));
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
                    _ => return Err(None),
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
                #[cfg(feature = "f16")]
                let f_16;
                let f_32;
                match () {
                    #[cfg(feature = "f16")]
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
