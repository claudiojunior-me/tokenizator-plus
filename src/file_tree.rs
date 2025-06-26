use std::fs;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};
use glob::Pattern;
use crate::{log_debug, log_warn};
use tiktoken_rs::cl100k_base_singleton;
use tokio::sync::mpsc::UnboundedSender;
use serde::Serialize;

fn is_ignored(entry: &DirEntry, root_path: &Path, ignore_patterns: &[Pattern]) -> bool {
    if let Ok(relative_path) = entry.path().strip_prefix(root_path) {
        for pattern in ignore_patterns {
            if pattern.matches_path(relative_path) {
                log_debug!("Ignoring {} due to pattern {}", relative_path.display(), pattern);
                return true;
            }
        }
    }
    false
}

#[derive(Serialize, Clone, Debug)]
pub struct Progress {
    pub processed: usize,
    pub total: usize,
}

fn generate_tree_and_content_internal(
    root_path: &Path,
    ignore_patterns_str: &[String],
    progress_tx: Option<&UnboundedSender<Progress>>,
) -> Result<String, std::io::Error> {
    log_debug!(
        "Scanning {} with patterns {:?}",
        root_path.display(),
        ignore_patterns_str
    );
    let ignore_patterns: Vec<Pattern> = ignore_patterns_str
        .iter()
        .filter_map(|s| Pattern::new(s).ok())
        .collect();

    let mut final_output = String::new();
    let mut file_paths = Vec::new();

    let mut tree_display = String::new();
    tree_display.push_str(".\n");

    let walker = WalkDir::new(root_path)
        .into_iter()
        .filter_entry(|e| !is_ignored(e, root_path, &ignore_patterns));

    for entry_result in walker {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.path() == root_path {
            continue;
        }

        let path = entry.path();
        let relative_path = path.strip_prefix(root_path).unwrap_or(path);

        if path.is_file() {
            file_paths.push(path.to_path_buf());
        }

        let depth = relative_path.components().count();
        let indent = "  ".repeat(depth);
        let prefix = "├── ";
        let file_name = relative_path.file_name().unwrap_or_default().to_string_lossy();
        tree_display.push_str(&format!("{}{}{}\n", indent, prefix, file_name));
    }

    final_output.push_str(&tree_display);
    final_output.push_str("\n\n");

    let total_files = file_paths.len();
    if let Some(tx) = progress_tx {
        tx.send(Progress { processed: 0, total: total_files }).ok();
    }

    for (idx, path) in file_paths.into_iter().enumerate() {
        let display_path = path.strip_prefix(root_path).unwrap_or(&path);
        let display_path_str = format!("/{}", display_path.display());

        final_output.push_str("--------------------------------------------------\n");
        final_output.push_str(&format!("{}\n", display_path_str));
        final_output.push_str("--------------------------------------------------\n\n");

        log_debug!("Reading {}", display_path_str);
        match fs::read_to_string(&path) {
            Ok(content) => {
                let total_lines = content.lines().count();
                let max_width = if total_lines == 0 { 1 } else { total_lines.to_string().len() };

                for (i, line) in content.lines().enumerate() {
                    let line_number = i + 1;
                    let formatted_line = format!(
                        "{:>width$} | {}\n",
                        line_number,
                        line,
                        width = max_width
                    );
                    final_output.push_str(&formatted_line);
                }
            },
            Err(e) => {
                log_warn!("Failed to read {}: {}", path.display(), e);
                final_output.push_str("[Error: could not read file. It may be binary.]\n");
            }
        }

        final_output.push_str("\n\n");

        if let Some(tx) = progress_tx {
            tx.send(Progress { processed: idx + 1, total: total_files }).ok();
        }
    }

    log_debug!("Finished scanning {}", root_path.display());
    Ok(final_output)
}

pub fn generate_tree_and_content(
    root_path: &Path,
    ignore_patterns_str: &[String],
) -> Result<String, std::io::Error> {
    generate_tree_and_content_internal(root_path, ignore_patterns_str, None)
}

pub fn generate_tree_and_content_with_progress(
    root_path: &Path,
    ignore_patterns_str: &[String],
    progress_tx: &UnboundedSender<Progress>,
) -> Result<String, std::io::Error> {
    generate_tree_and_content_internal(root_path, ignore_patterns_str, Some(progress_tx))
}


/// Conta tokens usando o modelo cl100k_base do tiktoken-rs.
pub fn count_tokens(text: &str) -> usize {
    let bpe = cl100k_base_singleton();
    bpe.encode_with_special_tokens(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    fn create_dir_with_file(lines: usize) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        let content: String = (1..=lines)
            .map(|i| format!("line{}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(&file_path, content).unwrap();
        dir
    }

    fn extract_file_lines(output: &str) -> Vec<String> {
        let marker = "/file.txt\n--------------------------------------------------\n\n";
        let start = output
            .find(marker)
            .map(|i| i + marker.len())
            .expect("file block start not found");
        output[start..]
            .lines()
            .take_while(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect()
    }

    #[test]
    fn line_numbers_align() {
        for &count in &[9usize, 10, 100] {
            let dir = create_dir_with_file(count);
            let output = generate_tree_and_content(dir.path(), &[]).unwrap();
            let lines = extract_file_lines(&output);
            assert_eq!(lines.len(), count);
            let width = count.to_string().len();
            for (i, line) in lines.iter().enumerate() {
                let expected = format!("{:>width$} |", i + 1, width = width);
                assert!(line.starts_with(&expected), "line '{}', count {}", line, count);
            }
        }
    }    

    #[test]
    fn ignore_patterns_exclude_files_and_dirs() {
        let dir = tempdir().expect("failed to create temp dir");
        let root = dir.path();

        // Files and directories to keep
        fs::write(root.join("keep.txt"), "keep").unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src").join("lib.rs"), "lib").unwrap();

        // Items that should be ignored
        fs::write(root.join("error.log"), "ignore").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules").join("mod.js"), "ignore").unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::write(root.join("target/debug/out.txt"), "ignore").unwrap();

        let patterns = vec!["*.log".to_string(), "node_modules".to_string(), "target".to_string()];
        let output = generate_tree_and_content(root, &patterns).unwrap();

        assert!(output.contains("keep.txt"));
        assert!(output.contains("src"));
        assert!(!output.contains("error.log"));
        assert!(!output.contains("node_modules"));
        assert!(!output.contains("target"));
        assert!(!output.contains("out.txt"));
        assert!(!output.contains("mod.js"));
    }
}
