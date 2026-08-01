use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::application::natural_order::natural_cmp;
use crate::application::ports::DirectoryWalker;
use crate::domain::source::SourceKind;

pub struct ExpandSources<'a> {
    pub walker: &'a dyn DirectoryWalker,
}

impl ExpandSources<'_> {
    /// Turns a mixed list of files and folders into the flat list of files
    /// this app can merge.
    ///
    /// Never fails as a whole. A directory that cannot be listed is logged and
    /// skipped, so one locked subfolder does not cost the user the rest of the
    /// tree -- the same rule `AddSources` follows for one unreadable file.
    ///
    /// Inputs keep their order relative to one another; the files found inside
    /// a folder are ordered naturally within that folder's subtree.
    pub fn execute(&self, inputs: &[PathBuf]) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for input in inputs {
            if self.walker.is_dir(input) {
                for path in self.collect(input) {
                    if seen.insert(path.clone()) {
                        result.push(path);
                    }
                }
            } else if seen.insert(input.clone()) {
                result.push(input.clone());
            }
        }

        result
    }

    /// Walks one tree and returns its supported files in natural order.
    fn collect(&self, root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut pending = vec![root.to_path_buf()];

        // An explicit stack rather than recursion: a deep tree is the user's
        // to choose, and it must not be able to overflow the call stack.
        while let Some(dir) = pending.pop() {
            let entries = match self.walker.read_dir(&dir) {
                Ok(entries) => entries,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        path = %dir.display(),
                        "directory could not be listed"
                    );
                    continue;
                }
            };

            for entry in entries {
                if is_hidden(&entry.path) {
                    continue;
                }

                if entry.is_dir {
                    pending.push(entry.path);
                } else if SourceKind::from_extension(&entry.path).is_some() {
                    files.push(entry.path);
                }
            }
        }

        // Sorting once at the end, rather than per directory, is what lets the
        // stack pop in any order and still produce file-manager order overall.
        files.sort_by(|left, right| natural_cmp(left, right));
        files
    }
}

/// Whether the entry's own name begins with a dot. Hidden files are almost
/// never material the user meant to merge, and a hidden directory can hold a
/// large tree of something else entirely.
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use crate::application::errors::WalkError;
    use crate::infrastructure::fake_engine::{file, subdir, FakeDirectoryWalker};

    use super::*;

    fn expand(walker: &FakeDirectoryWalker, inputs: &[&str]) -> Vec<String> {
        let inputs = inputs.iter().map(PathBuf::from).collect::<Vec<_>>();
        ExpandSources { walker }
            .execute(&inputs)
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn files_directly_inside_a_folder_are_collected() {
        let walker = FakeDirectoryWalker::new()
            .with_dir("/scans", vec![file("/scans/a.pdf"), file("/scans/b.png")]);

        assert_eq!(
            expand(&walker, &["/scans"]),
            ["/scans/a.pdf", "/scans/b.png"]
        );
    }

    #[test]
    fn subfolders_are_walked_to_any_depth() {
        let walker = FakeDirectoryWalker::new()
            .with_dir("/scans", vec![subdir("/scans/2024")])
            .with_dir("/scans/2024", vec![subdir("/scans/2024/q1")])
            .with_dir("/scans/2024/q1", vec![file("/scans/2024/q1/deep.pdf")]);

        assert_eq!(expand(&walker, &["/scans"]), ["/scans/2024/q1/deep.pdf"]);
    }

    #[test]
    fn unsupported_extensions_are_left_behind() {
        let walker = FakeDirectoryWalker::new().with_dir(
            "/scans",
            vec![
                file("/scans/notes.txt"),
                file("/scans/keep.pdf"),
                file("/scans/no-extension"),
            ],
        );

        assert_eq!(expand(&walker, &["/scans"]), ["/scans/keep.pdf"]);
    }

    #[test]
    fn dot_prefixed_files_and_folders_are_skipped() {
        let walker = FakeDirectoryWalker::new()
            .with_dir(
                "/scans",
                vec![
                    file("/scans/.hidden.pdf"),
                    subdir("/scans/.git"),
                    file("/scans/visible.pdf"),
                ],
            )
            .with_dir("/scans/.git", vec![file("/scans/.git/objects.pdf")]);

        assert_eq!(expand(&walker, &["/scans"]), ["/scans/visible.pdf"]);
    }

    #[test]
    fn results_come_back_in_natural_order_across_the_whole_tree() {
        let walker = FakeDirectoryWalker::new()
            .with_dir("/s", vec![subdir("/s/10"), subdir("/s/2")])
            .with_dir(
                "/s/10",
                vec![file("/s/10/page2.pdf"), file("/s/10/page10.pdf")],
            )
            .with_dir("/s/2", vec![file("/s/2/page1.pdf")]);

        assert_eq!(
            expand(&walker, &["/s"]),
            ["/s/2/page1.pdf", "/s/10/page2.pdf", "/s/10/page10.pdf"]
        );
    }

    #[test]
    fn ordering_spans_the_whole_tree_rather_than_one_directory_at_a_time() {
        // The final order interleaves the levels: `/s/c.pdf` is found while
        // listing `/s`, before either subfolder is opened, yet it sorts last.
        // Sorting each directory as it is popped could not produce this.
        let walker = FakeDirectoryWalker::new()
            .with_dir("/s", vec![subdir("/s/b"), subdir("/s/a"), file("/s/c.pdf")])
            .with_dir("/s/a", vec![file("/s/a/2.pdf")])
            .with_dir("/s/b", vec![file("/s/b/1.pdf")]);

        assert_eq!(
            expand(&walker, &["/s"]),
            ["/s/a/2.pdf", "/s/b/1.pdf", "/s/c.pdf"]
        );
    }

    #[test]
    fn a_symlinked_directory_is_never_descended_into() {
        // The link is listed as a file entry, so the walk leaves it alone even
        // though the target is a registered directory. Following it is how a
        // link cycle would hang the scan.
        let walker = FakeDirectoryWalker::new()
            .with_dir("/scans", vec![file("/scans/link"), file("/scans/real.pdf")])
            .with_dir("/scans/link", vec![file("/scans/link/looped.pdf")]);

        assert_eq!(expand(&walker, &["/scans"]), ["/scans/real.pdf"]);
    }

    #[test]
    fn an_unreadable_subfolder_is_skipped_and_the_walk_continues() {
        // `/scans/locked` is registered last so the stack pops it first: the
        // only file in the tree is still waiting behind the failure, and a walk
        // that gave up there would return nothing at all.
        let walker = FakeDirectoryWalker::new()
            .with_dir(
                "/scans",
                vec![subdir("/scans/other"), subdir("/scans/locked")],
            )
            .with_dir("/scans/other", vec![file("/scans/other/reachable.pdf")])
            .with_failure(
                "/scans/locked",
                WalkError::Unreadable {
                    path: "/scans/locked".into(),
                    reason: "permission denied".into(),
                },
            );

        assert_eq!(expand(&walker, &["/scans"]), ["/scans/other/reachable.pdf"]);
    }

    #[test]
    fn duplicate_paths_are_returned_once() {
        let walker = FakeDirectoryWalker::new().with_dir("/scans", vec![file("/scans/a.pdf")]);

        assert_eq!(
            expand(&walker, &["/scans", "/scans", "/scans/a.pdf"]),
            ["/scans/a.pdf"]
        );
    }

    #[test]
    fn inputs_that_are_not_directories_pass_through_untouched() {
        // Including unsupported ones: deciding what to do with a lone .txt is
        // `AddSources`'s job, and it already records or skips it.
        let walker = FakeDirectoryWalker::new();

        assert_eq!(
            expand(&walker, &["/a/report.pdf", "/a/notes.txt"]),
            ["/a/report.pdf", "/a/notes.txt"]
        );
    }

    #[test]
    fn input_order_is_preserved_between_separate_inputs() {
        let walker = FakeDirectoryWalker::new()
            .with_dir("/z", vec![file("/z/z.pdf")])
            .with_dir("/a", vec![file("/a/a.pdf")]);

        assert_eq!(expand(&walker, &["/z", "/a"]), ["/z/z.pdf", "/a/a.pdf"]);
    }

    #[test]
    fn a_folder_holding_nothing_supported_expands_to_nothing() {
        let walker = FakeDirectoryWalker::new().with_dir("/scans", vec![file("/scans/notes.txt")]);

        assert!(expand(&walker, &["/scans"]).is_empty());
    }
}
