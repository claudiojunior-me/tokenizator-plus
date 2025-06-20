use std::fs;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};
use glob::Pattern;
use tiktoken_rs::cl100k_base_singleton;

fn is_ignored(entry: &DirEntry, root_path: &Path, ignore_patterns: &[Pattern]) -> bool {
    if let Ok(relative_path) = entry.path().strip_prefix(root_path) {
        for pattern in ignore_patterns {
            if pattern.matches_path(relative_path) {
                return true;
            }
        }
    }
    false
}

pub fn generate_tree_and_content(
    root_path: &Path,
    ignore_patterns_str: &[String],
) -> Result<String, std::io::Error> {
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

        // Skip the root folder itself
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


    for path in file_paths {
        // Use the same method to obtain the relative path for display and prepend '/'.
        let display_path = path.strip_prefix(root_path).unwrap_or(&path);
        let display_path_str = format!("/{}", display_path.display());

        final_output.push_str("--------------------------------------------------\n");
        final_output.push_str(&format!("{}\n", display_path_str));
        final_output.push_str("--------------------------------------------------\n\n");
        
        match fs::read_to_string(&path) {
            Ok(content) => {
                // 1. Determine the padding width for line numbers.
                let total_lines = content.lines().count();
                // If the file is empty the width is 1; otherwise use the number of digits.
                let max_width = if total_lines == 0 { 1 } else { total_lines.to_string().len() };

                // 2. Iterate over each line, obtaining the index and content.
                for (i, line) in content.lines().enumerate() {
                    let line_number = i + 1;

                    // 3. Format the line with the number right-aligned using `{:>width$}`.
                    let formatted_line = format!(
                        "{:>width$} | {}\n",
                        line_number,
                        line,
                        width = max_width
                    );
                    final_output.push_str(&formatted_line);
                }
            },
            Err(_) => {
                // Preserve the error message for binary or unreadable files.
                final_output.push_str("[Error: could not read file. It may be binary.]\n");
            }
        }
        
        final_output.push_str("\n\n");
    }

    Ok(final_output)
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
