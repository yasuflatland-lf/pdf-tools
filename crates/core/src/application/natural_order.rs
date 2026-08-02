use std::cmp::Ordering;
use std::path::{Component, Path};

/// Orders two paths the way a file manager does.
///
/// Components compare left to right, and inside a component a run of digits
/// compares as a number rather than as text, so `page2` precedes `page10`.
/// Text runs compare case-insensitively first, because byte order would file
/// every capitalised name ahead of every lowercase one and that is not the
/// order the user saw in the picker.
pub fn natural_cmp(left: &Path, right: &Path) -> Ordering {
    let mut left_parts = left.components().peekable();
    let mut right_parts = right.components().peekable();

    loop {
        match (left_parts.next(), right_parts.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_part), Some(right_part)) => {
                let left_text = with_separator(&left_part, left_parts.peek().is_some());
                let right_text = with_separator(&right_part, right_parts.peek().is_some());

                let ordering = compare_component(&left_text, &right_text);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

/// Renders one component for comparison, keeping its separator when another
/// component follows it.
///
/// That separator is what puts `a.pdf` ahead of `a/b.pdf`: `.` sorts before
/// `/`, so a file sorts ahead of the folder whose name it extends. Doing it
/// by appending the separator, rather than by special-casing a shared prefix,
/// is what keeps the result a total order — a special case for `a` against
/// `a.pdf` alone would rank `a.pdf` first, `a` before `a-b`, and `a-b` before
/// `a.pdf`, a cycle `sort_by` is entitled to panic on.
fn with_separator(part: &Component<'_>, followed: bool) -> String {
    let mut text = part.as_os_str().to_string_lossy().into_owned();
    if followed {
        text.push('/');
    }
    text
}

/// Compares one component by walking it as alternating text and digit runs.
fn compare_component(left: &str, right: &str) -> Ordering {
    let mut left_rest = left;
    let mut right_rest = right;

    while !left_rest.is_empty() && !right_rest.is_empty() {
        let left_is_digits = left_rest.starts_with(|c: char| c.is_ascii_digit());
        let right_is_digits = right_rest.starts_with(|c: char| c.is_ascii_digit());

        let (left_run, left_tail) = split_run(left_rest, left_is_digits);
        let (right_run, right_tail) = split_run(right_rest, right_is_digits);

        let ordering = if left_is_digits && right_is_digits {
            compare_digit_runs(left_run, right_run)
        } else {
            left_run
                .to_lowercase()
                .cmp(&right_run.to_lowercase())
                .then_with(|| left_run.cmp(right_run))
        };

        if ordering != Ordering::Equal {
            return ordering;
        }

        left_rest = left_tail;
        right_rest = right_tail;
    }

    left_rest.len().cmp(&right_rest.len())
}

/// Splits off the leading run of digits, or the leading run of non-digits.
fn split_run(text: &str, digits: bool) -> (&str, &str) {
    let end = text
        .find(|c: char| c.is_ascii_digit() != digits)
        .unwrap_or(text.len());
    text.split_at(end)
}

/// Compares two runs of digits by value.
///
/// Length-then-lexicographic on the zero-stripped runs is exactly numeric
/// order, and unlike parsing it cannot overflow on an absurdly long run. A
/// tie falls back to the raw length so `7` sorts ahead of `007` instead of
/// landing in whichever order the input happened to arrive in.
fn compare_digit_runs(left: &str, right: &str) -> Ordering {
    let left_value = left.trim_start_matches('0');
    let right_value = right.trim_start_matches('0');

    left_value
        .len()
        .cmp(&right_value.len())
        .then_with(|| left_value.cmp(right_value))
        .then_with(|| left.len().cmp(&right.len()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn sorted(paths: &[&str]) -> Vec<String> {
        let mut paths = paths.iter().map(PathBuf::from).collect::<Vec<_>>();
        paths.sort_by(|left, right| natural_cmp(left, right));
        paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn digit_runs_compare_as_numbers() {
        assert_eq!(
            sorted(&["/s/page10.pdf", "/s/page2.pdf", "/s/page1.pdf"]),
            ["/s/page1.pdf", "/s/page2.pdf", "/s/page10.pdf"]
        );
    }

    #[test]
    fn leading_zeros_do_not_change_the_number() {
        // Same value, so the shorter spelling settles the tie and the order is
        // stable rather than arbitrary.
        assert_eq!(
            sorted(&["/s/p007.pdf", "/s/p7.pdf", "/s/p08.pdf"]),
            ["/s/p7.pdf", "/s/p007.pdf", "/s/p08.pdf"]
        );
    }

    #[test]
    fn components_are_compared_left_to_right() {
        assert_eq!(
            sorted(&["/s/2024/b.pdf", "/s/2023/z.pdf", "/s/10/a.pdf"]),
            ["/s/10/a.pdf", "/s/2023/z.pdf", "/s/2024/b.pdf"]
        );
    }

    #[test]
    fn a_shorter_path_precedes_the_longer_one_it_prefixes() {
        assert_eq!(
            sorted(&["/s/a/b.pdf", "/s/a.pdf"]),
            ["/s/a.pdf", "/s/a/b.pdf"]
        );
    }

    #[test]
    fn a_file_and_the_sibling_folders_around_it_form_a_total_order() {
        // `sort_by` may panic when the comparison is not transitive, so the
        // rule that puts `a.pdf` before `a/` has to hold up next to a third
        // name that falls between them.
        assert_eq!(
            sorted(&["/s/a/y.pdf", "/s/a.pdf", "/s/a-b/x.pdf"]),
            ["/s/a-b/x.pdf", "/s/a.pdf", "/s/a/y.pdf"]
        );
        assert_eq!(
            natural_cmp(Path::new("/s/a-b/x.pdf"), Path::new("/s/a.pdf")),
            Ordering::Less
        );
        assert_eq!(
            natural_cmp(Path::new("/s/a.pdf"), Path::new("/s/a/y.pdf")),
            Ordering::Less
        );
        assert_eq!(
            natural_cmp(Path::new("/s/a-b/x.pdf"), Path::new("/s/a/y.pdf")),
            Ordering::Less
        );
    }

    #[test]
    fn text_compares_case_insensitively_first() {
        // Byte order would put every capital ahead of every lowercase letter,
        // which is not what the picker showed the user.
        assert_eq!(
            sorted(&["/s/beta.pdf", "/s/Alpha.pdf", "/s/gamma.pdf"]),
            ["/s/Alpha.pdf", "/s/beta.pdf", "/s/gamma.pdf"]
        );
    }

    #[test]
    fn case_still_breaks_an_otherwise_equal_tie() {
        assert_eq!(sorted(&["/s/a.pdf", "/s/A.pdf"]), ["/s/A.pdf", "/s/a.pdf"]);
    }

    #[test]
    fn mixed_text_and_digits_alternate_correctly() {
        assert_eq!(
            sorted(&["/s/x2y10.pdf", "/s/x2y2.pdf", "/s/x10y1.pdf"]),
            ["/s/x2y2.pdf", "/s/x2y10.pdf", "/s/x10y1.pdf"]
        );
    }
}
