#[derive(Debug, PartialEq)]
pub enum TextDiff {
    Removed(String),
    Unchanged(String),
    New(String),
}

pub fn calculate_text_diff(old: &str, new: &str) -> Vec<TextDiff> {
    let mut text_changes = vec![];

    // diff::chars() gives us the diff char by char, so we regroup consecutive
    // character changes into strings for simpler manipulation.
    for diff in diff::chars(old, new) {
        match diff {
            diff::Result::Left(l) => {
                if let Some(TextDiff::Removed(old)) = text_changes.last_mut() {
                    old.push(l)
                } else {
                    text_changes.push(TextDiff::Removed(String::from(l)))
                }
            },
            diff::Result::Both(b, _) => {
                if let Some(TextDiff::Unchanged(unchanged)) = text_changes.last_mut() {
                    unchanged.push(b)
                } else {
                    text_changes.push(TextDiff::Unchanged(String::from(b)))
                }
            },
            diff::Result::Right(r) => {
                if let Some(TextDiff::New(new)) = text_changes.last_mut() {
                    new.push(r)
                } else {
                    text_changes.push(TextDiff::New(String::from(r)))
                }
            },
        }
    }
    text_changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_diff_works() {
        let diff_changes = calculate_text_diff("old_file.txt", "newer_file.md");
        assert_eq!(diff_changes, &[
            // NOTE: removed is always first
            TextDiff::Removed("old".to_owned()),
            TextDiff::New("newer".to_owned()),
            TextDiff::Unchanged("_file.".to_owned()),
            TextDiff::Removed("txt".to_owned()),
            TextDiff::New("md".to_owned()),
        ]);
    }
}
