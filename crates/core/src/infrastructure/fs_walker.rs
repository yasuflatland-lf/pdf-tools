use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::application::errors::WalkError;
use crate::application::ports::{DirectoryWalker, WalkEntry};

/// The real filesystem behind `DirectoryWalker`.
pub struct StdFsWalker;

impl DirectoryWalker for StdFsWalker {
    fn is_dir(&self, path: &Path) -> bool {
        // Follows symlinks, deliberately: the user picking a linked folder in
        // the dialog means that folder. Only descending is restricted.
        path.is_dir()
    }

    fn read_dir(&self, dir: &Path) -> Result<Vec<WalkEntry>, WalkError> {
        let entries = fs::read_dir(dir).map_err(|error| walk_error(dir, &error))?;

        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| walk_error(dir, &error))?;
            let path = entry.path();
            // `symlink_metadata` does not follow the link, so a directory
            // symlink reports as a file and the walk never descends into it.
            // An entry that cannot be stated at all is treated as a file: it
            // is then filtered by extension rather than recursed into.
            let is_dir = fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_dir());
            result.push(WalkEntry { path, is_dir });
        }

        Ok(result)
    }
}

fn walk_error(dir: &Path, error: &std::io::Error) -> WalkError {
    if error.kind() == ErrorKind::NotFound {
        WalkError::Missing {
            path: dir.to_path_buf(),
        }
    } else {
        WalkError::Unreadable {
            path: dir.to_path_buf(),
            reason: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn read_dir_reports_files_and_directories_apart() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.pdf"), b"x").unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();

        let mut entries = StdFsWalker.read_dir(root.path()).unwrap();
        entries.sort_by(|left, right| left.path.cmp(&right.path));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, root.path().join("a.pdf"));
        assert!(!entries[0].is_dir);
        assert_eq!(entries[1].path, root.path().join("nested"));
        assert!(entries[1].is_dir);
    }

    #[cfg(unix)]
    #[test]
    fn is_dir_follows_a_symlink_to_a_directory() {
        // A folder the user picked deliberately has to work even when it is a
        // link, which is why the entry rule and this rule differ.
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        fs::create_dir(&target).unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(StdFsWalker.is_dir(&link));
        assert!(StdFsWalker.is_dir(&target));
        assert!(!StdFsWalker.is_dir(&root.path().join("absent")));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_directory_entry_is_not_reported_as_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, root.path().join("link")).unwrap();

        let entries = StdFsWalker.read_dir(root.path()).unwrap();
        let link = entries
            .iter()
            .find(|entry| entry.path.ends_with("link"))
            .expect("the link should be listed");

        assert!(!link.is_dir);
    }

    #[test]
    fn a_missing_directory_reports_missing() {
        let root = tempfile::tempdir().unwrap();

        assert!(matches!(
            StdFsWalker
                .read_dir(&root.path().join("absent"))
                .unwrap_err(),
            WalkError::Missing { .. }
        ));
    }

    #[test]
    fn a_file_given_where_a_directory_was_expected_reports_unreadable() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.pdf");
        fs::write(&path, b"x").unwrap();

        assert!(matches!(
            StdFsWalker.read_dir(&path).unwrap_err(),
            WalkError::Unreadable { .. }
        ));
    }
}
