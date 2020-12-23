/// A CBOR number represented.
///
/// This goes from `-(u64::MAX + 1)` up to `u64::MAX` (inclusive).
#[allow(nonstandard_style)]
#[derive(Clone, Debug)]
pub struct i65 {
    /// In the English sense, that is: `true` iff `self < 0`.
    pub negative: bool,
    /// Absolute value of `self`. When `abs = 0`, `negative = true` would not
    /// make sense; this niche is thus used to represent the number
    /// `-(u64::MAX + 1)`, whose absolute value does not fit in a `u64`.
    pub abs: u64,
}

impl i65 {
    pub const ZERO: Self = Self {
        abs: 0,
        negative: false,
    };

    pub const MIN: Self = Self {
        negative: true,
        abs: 0,
    };

    pub const MAX: Self = Self {
        abs: ::core::u64::MAX,
        negative: false,
    };
}

#[inline]
const fn i65_to_i128(i65: i65) -> i128 {
    match i65 {
        i65::MIN => -(::core::u64::MAX as i128 + 1),
        i65 {
            abs,
            negative: false,
        } => abs as i128,
        i65 {
            abs,
            negative: true,
        } => -(abs as i128),
    }
}

impl From<i65> for i128 {
    #[inline]
    fn from(i65: i65) -> i128 {
        i65_to_i128(i65)
    }
}

impl TryFrom<i128> for i65 {
    type Error = ();

    #[inline]
    fn try_from(x: i128) -> Result<i65, Self::Error> {
        const MIN: i128 = i65_to_i128(i65::MIN);
        const MIN_PLUS_1: i128 = i65_to_i128(i65::MIN) + 1;
        const MAX: i128 = i65_to_i128(i65::MAX);
        Ok(match x {
            MIN => i65::MIN,
            MIN_PLUS_1..0 => i65 {
                negative: true,
                abs: (-x) as u64,
            },
            0..=MAX => i65 {
                negative: false,
                abs: x as u64,
            },
            _ => return Err(()),
        })
    }
}

impl From<u64> for i65 {
    #[inline]
    fn from(abs: u64) -> i65 {
        i65 {
            negative: false,
            abs,
        }
    }
}

impl From<i64> for i65 {
    #[inline]
    fn from(x: i64) -> i65 {
        use ::core::i64;
        match x {
            i64::MIN => i65 {
                negative: true,
                abs: (i64::MAX as u64) + 1,
            },
            i64::MIN..0 => i65 {
                negative: true,
                abs: x.wrapping_neg() as _,
            },
            0 => i65 {
                negative: false,
                abs: 0,
            },
            1..=i64::MAX => i65 {
                negative: false,
                abs: x as _,
            },
        }
    }
}

impl_From! {
    [u8, u16, u32, usize] => u64
}
impl_From! {
    [i8, i16, i32, isize] => i64
}
macro_rules! impl_From {(
    [$($xN:ty),*] => $x64:ty
) => (
    $(
        impl From<$xN> for i65 {
            #[inline]
            fn from (value: $xN) -> Self {
                Self::from(value as $x64)
            }
        }
    )*
)}
use impl_From;
