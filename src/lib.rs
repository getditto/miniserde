#![cfg_attr(doc, feature(external_doc, doc_cfg))]
#![cfg_attr(doc, doc(include = "../README.md"))]
#![allow(
    clippy::needless_doctest_main,
    clippy::vec_init_then_push,
    // Regression causing false positives:
    // https://github.com/rust-lang/rust-clippy/issues/5343
    clippy::useless_transmute,
    // Clippy bug: https://github.com/rust-lang/rust-clippy/issues/5704
    clippy::unnested_or_patterns,
    // We support older compilers.
    clippy::manual_range_contains,
    // Pedantic.
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::checked_conversions,
    clippy::doc_markdown,
    clippy::enum_glob_use,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::redundant_else,
    clippy::shadow_unrelated,
    clippy::single_match_else,
    clippy::too_many_lines,
)]
#![deny(rust_2018_idioms)]
#![allow(explicit_outlives_requirements)]

extern crate self as miniserde_ditto;

#[doc(hidden)]
#[macro_export]
macro_rules! __err__ {(
    $($args:tt)*
) => ({
    if ::core::option_env!("MINISERDE_DEBUG_ERRORS") == Some("1") {
        ::std::eprintln!("Serde error: {}", ::core::format_args!($($args)*));
    }
    return $crate::ResultLike::ERROR;
})}
macro_rules! err {(
    $($args:tt)*
) => (
    $crate::__::err! { $($args)* }
)}

#[doc(hidden)]
pub trait ResultLike {
    const ERROR: Self;
}
impl<T> ResultLike for Result<T> {
    const ERROR: Self = Err(Error);
}
impl<T> ResultLike for Option<T> {
    const ERROR: Self = None;
}
impl<T, E> ResultLike for Result<T, Option<E>> {
    const ERROR: Self = Err(None);
}
impl ResultLike for Error {
    const ERROR: Self = Error;
}

#[doc(hidden)]
pub use ::derives::*;

/// Not public API.
#[doc(hidden)]
pub mod __private;

/// Not public API.
#[doc(hidden)]
pub use __private as __;

mod aliased_box;

#[macro_use]
mod careful;

#[macro_use]
mod place;

mod error;

#[cfg(feature = "cbor")]
#[cfg_attr(doc, doc(cfg(feature = "cbor")))]
pub mod cbor;
pub mod de;
#[cfg(feature = "json")]
#[cfg_attr(doc, doc(cfg(feature = "json")))]
pub mod json;
pub mod ser;

#[doc(inline)]
pub use crate::de::Deserialize;
pub use crate::error::{Error, Result};
#[doc(inline)]
pub use crate::ser::Serialize;

make_place!(Place);

#[allow(non_camel_case_types)]
struct private;

macro_rules! with_Ns {( $($rules:tt)* ) => (
    macro_rules! __helper__ { $($rules)* }
    __helper__! {
        00,
        01, 02, 03, 04, 05, 06, 07, 08,
        09, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32,
    }
)}
pub(in crate) use with_Ns;
