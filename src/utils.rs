//! Utils for the script.

use std::error::Error;
use std::fs::metadata;
use std::path::Path;
use std::time::SystemTime;

use chrono::DateTime;
use chrono::Utc;

/// Get the last time a file has been modified.
pub fn get_last_modified(path: &Path) -> Result<DateTime<Utc>, Box<dyn Error>> {
    Ok(DateTime::from_timestamp_millis(
        metadata(path)?
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis()
            .try_into()
            .unwrap(),
    )
    .unwrap())
}

/// Iterate starting from the specified starting point.
/// For instance if you a=have the array [a, b, c] and you set the starting
/// point to b, then the iterator will produce the value c, a in this order,
/// ignoring the starting point.
pub fn into_iter_from<'a, T>(
    values: Vec<&'a T>,
    start: &'a Option<T>,
    reverse: bool,
) -> Box<dyn Iterator<Item = &'a T> + 'a>
where
    T: PartialEq + Clone,
{
    if let Some(start) = start {
        if reverse {
            Box::new(
                values
                    .into_iter()
                    .cycle()
                    .skip_while(|r| **r != start.clone())
                    .take_while(|r| **r != start.clone()),
            )
        } else {
            Box::new(
                values
                    .into_iter()
                    .rev()
                    .cycle()
                    .skip_while(|r| **r != start.clone())
                    .take_while(|r| **r != start.clone()),
            )
        }
    } else if reverse {
        Box::new(values.into_iter().rev())
    } else {
        Box::new(values.into_iter())
    }
}
