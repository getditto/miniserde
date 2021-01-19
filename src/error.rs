use std::fmt::{self, Display};

/// Error type when deserialization fails.
///
/// Miniserde errors contain no information about what went wrong. **If you need
/// more than no information, use Serde.**
///
/// If you really want to have some hacky way to access more info about some
/// serde failure, if you compile this crate with the following env var:
///
///   - **`MINISERDE_DEBUG_ERRORS=1`**
///
/// then, more explicit error messages will be printed to the `stderr` when
/// encountered.
#[derive(Copy, Clone, Debug)]
pub struct Error;

/// Result type returned by deserialization functions.
pub type Result<Ok, Err = Error> = std::result::Result<Ok, Err>;

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("miniserde error")
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "miniserde error"
    }
}
