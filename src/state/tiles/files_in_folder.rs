use ::std::ffi::OsString;

use crate::state::files::{FileOrFolder, Folder};
use crate::state::Metric;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileType {
    File,
    Folder,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub name: OsString,
    pub size: u128,
    pub descendants: Option<u64>,
    pub file_count: u64,
    pub percentage: f64, // 1.0 is 100% (0.5 is 50%, etc.)
    pub file_type: FileType,
}

fn calculate_percentage(
    value: u128,
    total_value: u128,
    total_files_in_parent: usize,
) -> f64 {
    if value == 0 && total_value == 0 {
        // if all files in the folder have a zero value in the active metric,
        // we'll want to display them all as the same size
        1.0 / total_files_in_parent as f64
    } else {
        value as f64 / total_value as f64
    }
}

fn metric_value(metric: Metric, file: &FileMetadata) -> u128 {
    match metric {
        Metric::Size => file.size,
        Metric::Count => file.file_count as u128,
    }
}

pub fn files_in_folder(folder: &Folder, offset: usize, metric: Metric) -> Vec<FileMetadata> {
    let mut files = Vec::new();
    let total_metric_value = match metric {
        Metric::Size => folder.size,
        Metric::Count => folder.recursive_file_count as u128,
    };
    for (name, file_or_folder) in &folder.contents {
        files.push({
            let size = file_or_folder.size();
            let file_count = file_or_folder.file_count();
            let name = name.clone();
            let (descendants, file_type) = match file_or_folder {
                FileOrFolder::Folder(folder) => (Some(folder.num_descendants), FileType::Folder),
                FileOrFolder::File(_file) => (None, FileType::File),
            };
            let percentage = calculate_percentage(
                match metric {
                    Metric::Size => size,
                    Metric::Count => file_count as u128,
                },
                total_metric_value,
                folder.contents.len(),
            );
            FileMetadata {
                size,
                name,
                descendants,
                file_count,
                percentage,
                file_type,
            }
        });
    }
    files.sort_by(|a, b| {
        let a_metric_value = metric_value(metric, a);
        let b_metric_value = metric_value(metric, b);
        if a_metric_value == b_metric_value {
            a.name.partial_cmp(&b.name).expect("could not compare name")
        } else {
            b_metric_value.cmp(&a_metric_value)
        }
    });
    if offset > 0 {
        let removed_items = files.drain(..offset);
        let number_of_files_without_removed_contents = folder.contents.len() - removed_items.len();
        let removed_metric_value = removed_items.fold(0, |acc, file| acc + metric_value(metric, &file));
        let total_without_removed_items = total_metric_value - removed_metric_value;
        for i in 0..files.len() {
            files[i].percentage = calculate_percentage(
                metric_value(metric, &files[i]),
                total_without_removed_items,
                number_of_files_without_removed_contents,
            );
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use crate::state::files::Folder;
    use crate::state::Metric;

    use super::files_in_folder;

    fn folder_with_entries() -> Folder {
        let mut folder = Folder::new(&PathBuf::from("/tmp/base"));
        folder.add_file(PathBuf::from("a.txt"), 10);
        folder.add_folder(PathBuf::from("empty"));
        folder.add_file(PathBuf::from("nested/b.txt"), 20);
        folder
    }

    #[test]
    fn count_mode_sorts_by_recursive_file_count() {
        let files = files_in_folder(&folder_with_entries(), 0, Metric::Count);

        let file_names: Vec<OsString> = files.into_iter().map(|file| file.name).collect();
        assert_eq!(
            file_names,
            vec![
                OsString::from("a.txt"),
                OsString::from("nested"),
                OsString::from("empty"),
            ]
        );
    }

    #[test]
    fn count_mode_uses_equal_split_when_all_children_have_zero_files() {
        let mut folder = Folder::new(&PathBuf::from("/tmp/base"));
        folder.add_folder(PathBuf::from("empty-a"));
        folder.add_folder(PathBuf::from("empty-b"));

        let files = files_in_folder(&folder, 0, Metric::Count);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].percentage, 0.5);
        assert_eq!(files[1].percentage, 0.5);
    }

    #[test]
    fn count_mode_keeps_zero_file_entries_at_zero_when_siblings_have_files() {
        let files = files_in_folder(&folder_with_entries(), 0, Metric::Count);

        let empty = files
            .into_iter()
            .find(|file| file.name == OsString::from("empty"))
            .expect("expected empty folder");
        assert_eq!(empty.file_count, 0);
        assert_eq!(empty.percentage, 0.0);
    }
}
