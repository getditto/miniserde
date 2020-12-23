use crate::{
    de::{self, VisitorSlot},
    error::Result,
};

impl dyn VisitorSlot {
    pub fn ignore() -> &'static mut dyn VisitorSlot {
        Box::leak(Box::new(Ignore))
    }
}

struct Ignore;

// #[with(dyn_safe = true)]
impl VisitorSlot for Ignore {
    fn write_null(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_boolean(&mut self, _: bool) -> Result<()> {
        Ok(())
    }

    fn write_string(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn write_integer(&mut self, _: i128) -> Result<()> {
        Ok(())
    }

    fn write_float(&mut self, _: f64) -> Result<()> {
        Ok(())
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
        Ok(with(Ok(self)))
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
        Ok(with(Ok(self)))
    }
}

impl de::Seq for Ignore {
    fn next_slot(&mut self) -> Result<&mut dyn VisitorSlot> {
        Ok(VisitorSlot::ignore())
    }

    // fn finish(&mut self) -> Result<()> {
    //     Ok(())
    // }
}

impl de::Map for Ignore {
    fn slot_at(&mut self, _: &str) -> Result<&mut dyn VisitorSlot> {
        Ok(VisitorSlot::ignore())
    }

    // fn finish(&mut self) -> Result<()> {
    //     Ok(())
    // }
}
