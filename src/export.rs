pub use std::borrow::Cow;
pub use std::boxed::Box;
pub use std::option::Option::{self, None, Some};
pub use std::result::Result::{Err, Ok};
pub use std::string::String;

pub use self::help::{Str as str, Usize as usize, U8 as u8};
mod help {
    pub type Str = str;
    pub type U8 = u8;
    pub type Usize = usize;
}
