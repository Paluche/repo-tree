//! Utils for the script.

use std::error::Error;
use std::fs::metadata;
use std::path::Path;

use jiff::Timestamp;

use crate::error::GetLastModifiedError;

/// Get the last time a file has been modified.
pub fn get_last_modified(path: &Path) -> Result<Timestamp, Box<dyn Error>> {
    Timestamp::try_from(metadata(path)?.modified()?)
        .map_err(|_| GetLastModifiedError(path.display().to_string()).into())
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

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::Duration;
    use std::time::SystemTime;

    use super::*;

    /// Collect `into_iter_from` over `[a, b, c]`.
    fn iter_from(start: Option<char>, reverse: bool) -> String {
        let values = ['a', 'b', 'c'];
        into_iter_from(values.iter().collect(), start.as_ref(), reverse)
            .collect()
    }

    #[test]
    fn into_iter_from_without_start_produces_every_value() {
        assert_eq!(iter_from(None, false), "abc");
        assert_eq!(iter_from(None, true), "cba");
    }

    #[test]
    fn into_iter_from_goes_forward_after_start() {
        assert_eq!(iter_from(Some('a'), false), "bc");
        assert_eq!(iter_from(Some('b'), false), "ca");
        assert_eq!(iter_from(Some('c'), false), "ab");
    }

    #[test]
    fn into_iter_from_goes_backward_after_start_when_reversed() {
        assert_eq!(iter_from(Some('a'), true), "cb");
        assert_eq!(iter_from(Some('b'), true), "ac");
        assert_eq!(iter_from(Some('c'), true), "ba");
    }

    #[test]
    fn into_iter_from_unknown_start_produces_every_value() {
        assert_eq!(iter_from(Some('z'), false), "abc");
        assert_eq!(iter_from(Some('z'), true), "cba");
    }

    #[test]
    fn into_iter_from_single_value_is_empty_from_itself() {
        let values = ['a'];
        assert_eq!(
            into_iter_from(values.iter().collect(), Some(&'a'), false).count(),
            0
        );
    }

    #[test]
    fn get_last_modified_reads_the_modification_time() {
        let dir = tempfile::tempdir().expect("cannot create temporary dir");
        let path = dir.path().join("file");
        let file = File::create(&path).expect("cannot create file");
        let time =
            SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_123);
        file.set_modified(time)
            .expect("cannot set modification time");

        let last_modified =
            get_last_modified(&path).expect("cannot get modification time");

        assert_eq!(last_modified.as_millisecond(), 1_700_000_123);
    }

    #[test]
    fn get_last_modified_of_missing_file_fails() {
        let dir = tempfile::tempdir().expect("cannot create temporary dir");

        assert!(get_last_modified(&dir.path().join("missing")).is_err());
    }
}
