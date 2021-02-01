pub use ::std::{
    self,
    borrow::Cow,
    boxed::Box,
    default::Default,
    ops::FnMut,
    option::Option::{self, None, Some},
    result::Result::{Err, Ok},
    string::String,
    stringify, vec,
    vec::Vec,
};

pub use crate::{__err__ as err, aliased_box::AliasedBox};

pub use self::help::{Str as str, Usize as usize};
mod help {
    pub type Str = str;
    pub type Usize = usize;
}

pub struct StrVisitor<F: FnMut(&str) -> crate::Result<()>>(pub F);

impl<F: FnMut(&str) -> crate::Result<()>> crate::de::Visitor for StrVisitor<F> {
    fn string(self: &'_ mut StrVisitor<F>, s: &'_ str) -> crate::Result<()> {
        (self.0)(s)
    }
}

#[derive(crate::Deserialize)]
pub struct Empty;
