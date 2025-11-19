use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fs::FileType;
use std::path::Path;
use std::path::PathBuf;

use codex_utils_string::take_bytes_at_char_boundary;
use tokio::fs;

const MAX_ENTRY_LENGTH: usize = 500;
const INDENTATION_SPACES: usize = 2;

pub fn default_offset() -> usize {
    1
}

pub fn default_limit() -> usize {
    25
}

pub fn default_depth() -> usize {
    2
}

pub async fn list_dir(
    dir_path: &Path,
    offset: usize,
    limit: usize,
    depth: usize,
) -> anyhow::Result<Vec<String>> {
    if offset == 0 {
        anyhow::bail!("offset must be a 1-indexed entry number");
    }

    if limit == 0 {
        anyhow::bail!("limit must be greater than zero");
    }

    if depth == 0 {
        anyhow::bail!("depth must be greater than zero");
    }

    if !dir_path.is_absolute() {
        anyhow::bail!("dir_path must be an absolute path");
    }

    let entries = list_dir_slice(dir_path, offset, limit, depth).await?;
    let mut output = Vec::with_capacity(entries.len() + 1);
    output.push(format!("Absolute path: {}", dir_path.display()));
    output.extend(entries);
    Ok(output)
}

async fn list_dir_slice(
    path: &Path,
    offset: usize,
    limit: usize,
    depth: usize,
) -> anyhow::Result<Vec<String>> {
    let mut entries = Vec::new();
    collect_entries(path, Path::new(""), depth, &mut entries).await?;

    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let start_index = offset - 1;
    if start_index >= entries.len() {
        anyhow::bail!("offset exceeds directory entry count");
    }

    let remaining_entries = entries.len() - start_index;
    let capped_limit = limit.min(remaining_entries);
    let end_index = start_index + capped_limit;
    let mut selected_entries = entries[start_index..end_index].to_vec();
    selected_entries.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    let mut formatted = Vec::with_capacity(selected_entries.len());

    for entry in &selected_entries {
        formatted.push(format_entry_line(entry));
    }

    if end_index < entries.len() {
        formatted.push(format!("More than {capped_limit} entries found"));
    }

    Ok(formatted)
}

async fn collect_entries(
    dir_path: &Path,
    relative_prefix: &Path,
    depth: usize,
    entries: &mut Vec<DirEntry>,
) -> anyhow::Result<()> {
    let mut queue = VecDeque::new();
    queue.push_back((dir_path.to_path_buf(), relative_prefix.to_path_buf(), depth));

    while let Some((current_dir, prefix, remaining_depth)) = queue.pop_front() {
        let mut read_dir = fs::read_dir(&current_dir)
            .await
            .map_err(|err| anyhow::anyhow!("failed to read directory: {err}"))?;

        let mut dir_entries = Vec::new();

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|err| anyhow::anyhow!("failed to read directory: {err}"))?
        {
            let file_type = entry
                .file_type()
                .await
                .map_err(|err| anyhow::anyhow!("failed to inspect entry: {err}"))?;

            let file_name = entry.file_name();
            let relative_path = if prefix.as_os_str().is_empty() {
                PathBuf::from(&file_name)
            } else {
                prefix.join(&file_name)
            };

            let display_name = format_entry_component(&file_name);
            let display_depth = prefix.components().count();
            let sort_key = format_entry_name(&relative_path);
            let kind = DirEntryKind::from(&file_type);
            dir_entries.push((
                entry.path(),
                relative_path,
                kind,
                DirEntry {
                    name: sort_key,
                    display_name,
                    depth: display_depth,
                    kind,
                },
            ));
        }

        dir_entries.sort_unstable_by(|a, b| a.3.name.cmp(&b.3.name));

        for (entry_path, relative_path, kind, dir_entry) in dir_entries {
            if kind == DirEntryKind::Directory && remaining_depth > 1 {
                queue.push_back((entry_path, relative_path, remaining_depth - 1));
            }
            entries.push(dir_entry);
        }
    }

    Ok(())
}

fn format_entry_name(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace("\\", "/");
    if normalized.len() > MAX_ENTRY_LENGTH {
        take_bytes_at_char_boundary(&normalized, MAX_ENTRY_LENGTH).to_string()
    } else {
        normalized
    }
}

fn format_entry_component(name: &OsStr) -> String {
    let normalized = name.to_string_lossy();
    if normalized.len() > MAX_ENTRY_LENGTH {
        take_bytes_at_char_boundary(&normalized, MAX_ENTRY_LENGTH).to_string()
    } else {
        normalized.to_string()
    }
}

fn format_entry_line(entry: &DirEntry) -> String {
    let indent = " ".repeat(entry.depth * INDENTATION_SPACES);
    let mut name = entry.display_name.clone();
    match entry.kind {
        DirEntryKind::Directory => name.push('/'),
        DirEntryKind::Symlink => name.push('@'),
        DirEntryKind::Other => name.push('?'),
        DirEntryKind::File => {}
    }
    format!("{indent}{name}")
}

#[derive(Clone)]
struct DirEntry {
    name: String,
    display_name: String,
    depth: usize,
    kind: DirEntryKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirEntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

impl From<&FileType> for DirEntryKind {
    fn from(file_type: &FileType) -> Self {
        if file_type.is_symlink() {
            DirEntryKind::Symlink
        } else if file_type.is_dir() {
            DirEntryKind::Directory
        } else if file_type.is_file() {
            DirEntryKind::File
        } else {
            DirEntryKind::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn lists_directory_entries() {
        let temp = tempdir().unwrap();
        let dir_path = temp.path();

        let sub_dir = dir_path.join("nested");
        tokio::fs::create_dir(&sub_dir).await.unwrap();

        let deeper_dir = sub_dir.join("deeper");
        tokio::fs::create_dir(&deeper_dir).await.unwrap();

        tokio::fs::write(dir_path.join("entry.txt"), b"content")
            .await
            .unwrap();
        tokio::fs::write(sub_dir.join("child.txt"), b"child")
            .await
            .unwrap();
        tokio::fs::write(deeper_dir.join("grandchild.txt"), b"grandchild")
            .await
            .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = dir_path.join("link");
            symlink(dir_path.join("entry.txt"), &link_path).unwrap();
        }

        let entries = list_dir_slice(dir_path, 1, 20, 3).await.unwrap();

        #[cfg(unix)]
        let expected = vec![
            "entry.txt".to_string(),
            "link@".to_string(),
            "nested/".to_string(),
            "  child.txt".to_string(),
            "  deeper/".to_string(),
            "    grandchild.txt".to_string(),
        ];

        #[cfg(not(unix))]
        let expected = vec![
            "entry.txt".to_string(),
            "nested/".to_string(),
            "  child.txt".to_string(),
            "  deeper/".to_string(),
            "    grandchild.txt".to_string(),
        ];

        assert_eq!(entries, expected);
    }
}
