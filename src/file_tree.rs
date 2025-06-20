use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use glob::Pattern;

/// Função auxiliar para verificar se uma entrada deve ser ignorada
fn is_ignored(entry: &DirEntry, root_path: &Path, ignore_patterns: &[Pattern]) -> bool {
    // Obtém o caminho relativo da entrada
    if let Ok(relative_path) = entry.path().strip_prefix(root_path) {
        // Verifica se o caminho relativo corresponde a algum dos padrões de exclusão
        for pattern in ignore_patterns {
            if pattern.matches_path(relative_path) {
                return true; // Se corresponder, deve ser ignorado
            }
        }
    }
    false // Se não corresponder a nenhum padrão, não deve ser ignorado
}

/// Gera a árvore de diretórios e o conteúdo concatenado dos arquivos.
pub fn generate_tree_and_content(
    root_path: &Path,
    ignore_patterns_str: &[String],
) -> Result<String, std::io::Error> {

    // Compila os padrões de string para padrões Glob, ignorando os que forem inválidos
    let ignore_patterns: Vec<Pattern> = ignore_patterns_str
        .iter()
        .filter_map(|s| Pattern::new(s).ok())
        .collect();

    let mut final_output = String::new();
    let mut file_paths = Vec::new();

    let mut tree_display = String::new();
    tree_display.push_str(".\n"); // Adiciona a raiz da árvore

    // Usa `filter_entry` para pular arquivos/diretórios que correspondem aos padrões
    let walker = WalkDir::new(root_path)
        .into_iter()
        .filter_entry(|e| !is_ignored(e, root_path, &ignore_patterns));

    for entry_result in walker {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue, // Pula entradas com erro de leitura
        };

        // Ignora a própria pasta raiz
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


    // --- Passagem 2: Ler e Anexar o Conteúdo de Cada Arquivo ---
    for path in file_paths {
        // --- MUDANÇA AQUI ---
        // Usa o mesmo método para obter o caminho relativo para exibição no cabeçalho.
        // Adicionamos uma barra inicial para o formato /caminho/relativo.py
        let display_path = path.strip_prefix(root_path).unwrap_or(&path);
        let display_path_str = format!("/{}", display_path.display());

        final_output.push_str("--------------------------------------------------\n");
        final_output.push_str(&format!("{}\n", display_path_str));
        final_output.push_str("--------------------------------------------------\n\n");
        
        match fs::read_to_string(&path) {
            Ok(content) => {
                // 1. Determina a largura necessária para o padding dos números de linha.
                let total_lines = content.lines().count();
                // Se o arquivo estiver vazio, a largura é 1. Senão, é o número de dígitos do total de linhas.
                let max_width = if total_lines == 0 { 1 } else { total_lines.to_string().len() };

                // 2. Itera sobre cada linha, obtendo o índice (i) e o conteúdo (line).
                for (i, line) in content.lines().enumerate() {
                    // O índice (i) começa em 0, então o número da linha é i + 1.
                    let line_number = i + 1;
                    
                    // 3. Formata a nova linha com o número alinhado à direita, o separador e o conteúdo.
                    //    `{:>width$}` é um formatador especial do Rust:
                    //    - `:` inicia a formatação.
                    //    - `>` significa alinhar à direita.
                    //    - `width$` usa uma variável (neste caso, `max_width`) para definir a largura do padding.
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
                // Mantém a mensagem de erro para arquivos binários ou ilegíveis.
                final_output.push_str("[Erro: Não foi possível ler o arquivo. Pode ser um arquivo binário.]\n");
            }
        }
        
        final_output.push_str("\n\n");
    }

    Ok(final_output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
}