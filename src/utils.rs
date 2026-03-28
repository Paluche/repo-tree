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
