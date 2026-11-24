//! Utils for the script.

use std::error::Error;
use std::fs::metadata;
use std::path::Path;
use std::time::SystemTime;

use chrono::DateTime;
use chrono::Utc;

use crate::error::GetLastModifiedError;

/// Get the last time a file has been modified.
pub fn get_last_modified(path: &Path) -> Result<DateTime<Utc>, Box<dyn Error>> {
    DateTime::from_timestamp_millis(
        metadata(path)?
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis()
            .try_into()?,
    )
    .ok_or(Box::new(GetLastModifiedError(path.display().to_string())))
}

/// Iterate over `values` starting after `start`, wrapping around, and
/// ignoring `start` itself.
///
/// For instance with `[a, b, c]` and `b` as starting point, the iterator
/// produces `c, a`, or `a, c` if `reverse` is set. Without a starting point,
/// or with one which is not in `values`, every value is produced, from the
/// first one (or the last one if `reverse` is set).
pub fn into_iter_from<'a, T>(
    mut values: Vec<&'a T>,
    start: Option<&T>,
    reverse: bool,
) -> impl Iterator<Item = &'a T>
where
    T: PartialEq,
{
    if reverse {
        values.reverse();
    }
    if let Some(index) =
        start.and_then(|start| values.iter().position(|v| *v == start))
    {
        values.rotate_left(index + 1);
        values.pop();
    }
    values.into_iter()
}
