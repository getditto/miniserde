use super::{Map, Seq, Visitor};
use crate::{Place, Result};

/// An efficient way of discarding data from a deserializer.
///
/// Think of this like a `{json,cbor}::Value` in that it can be deserialized
/// from any type, except that it does not store any information about the data
/// that gets deserialized.
///
/// When needing such a helper not as a `struct` field / `enum` variant, but
/// within an explicit [`Deserialize`][super::Deserialize] implementation,
/// keep in mind you can also directly use the [`Visitor::ignore()`](
/// trait.Visitor.html#method.ignore) helper, which is a tiny bit more
/// efficient (does not need to keep track of whether a deserialization has
/// occurred).
#[derive(Debug, Copy, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct IgnoredAny;

impl super::Deserialize for IgnoredAny {
    fn begin(out: &'_ mut Option<IgnoredAny>) -> &'_ mut dyn Visitor {
        impl Place<IgnoredAny> {
            fn mark_visited(&mut self) {
                self.out = Some(IgnoredAny);
            }
        }

        impl Visitor for Place<IgnoredAny> {
            fn null(&mut self) -> Result<()> {
                self.mark_visited();
                Ok(())
            }

            fn boolean(&mut self, _: bool) -> Result<()> {
                self.mark_visited();
                Ok(())
            }

            fn string(&mut self, _: &str) -> Result<()> {
                self.mark_visited();
                Ok(())
            }

            fn int(&mut self, _: i128) -> Result<()> {
                self.mark_visited();
                Ok(())
            }

            fn float(&mut self, _n: f64) -> Result<()> {
                self.mark_visited();
                Ok(())
            }

            fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
                self.mark_visited();
                Ok(Box::new(Ignore {}))
            }

            fn map(&mut self) -> Result<Box<dyn Map + '_>> {
                self.mark_visited();
                Ok(Box::new(Ignore {}))
            }
        }

        Place::new(out)
    }
}

struct Ignore {}

impl dyn 'static + Visitor {
    /// Creates a dummy `Visitor` that ignores the received values with a dummy
    /// success each time.
    pub fn ignore() -> &'static mut dyn Visitor {
        Box::leak(Box::new(Ignore {}))
    }
}

impl Visitor for Ignore {
    fn null(&mut self) -> Result<()> {
        Ok(())
    }

    fn boolean(&mut self, _: bool) -> Result<()> {
        Ok(())
    }

    fn string(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn int(&mut self, _: i128) -> Result<()> {
        Ok(())
    }

    fn float(&mut self, _: f64) -> Result<()> {
        Ok(())
    }

    fn seq(&mut self) -> Result<Box<dyn Seq + '_>> {
        Ok(Box::new(Ignore {}))
    }

    fn map(&mut self) -> Result<Box<dyn Map + '_>> {
        Ok(Box::new(Ignore {}))
    }
}

impl Seq for Ignore {
    fn element(&mut self) -> Result<&mut dyn Visitor> {
        Ok(Visitor::ignore())
    }

    fn finish(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}

impl Map for Ignore {
    fn val_with_key(
        &mut self,
        de_key: &mut dyn FnMut(Result<&mut dyn Visitor>) -> Result<()>,
    ) -> Result<&mut dyn Visitor> {
        de_key(Ok(Visitor::ignore()))?;
        Ok(Visitor::ignore())
    }

    fn finish(self: Box<Self>) -> Result<()> {
        Ok(())
    }
}
