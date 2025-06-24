#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::value::Value;
    use std::fs;
    use std::rc::Rc;
    use tempfile::TempDir;

    #[test]
    fn test_normalize_path() {
        let args = vec![Value::Text(Rc::from("path\\to\\file.txt"))];
        let result = native_normalize_path(args).unwrap();
        
        match result {
            Value::Text(path) => assert_eq!(path.as_ref(), "path/to/file.txt"),
            _ => panic!("Expected text result"),
        }
    }

    #[test]
    fn test_normalize_path_already_normalized() {
        let args = vec![Value::Text(Rc::from("path/to/file.txt"))];
        let result = native_normalize_path(args).unwrap();
        
        match result {
            Value::Text(path) => assert_eq!(path.as_ref(), "path/to/file.txt"),
            _ => panic!("Expected text result"),
        }
    }

    #[test]
    fn test_file_mtime() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let args = vec![Value::Text(Rc::from(file_path.to_string_lossy().as_ref()))];
        let result = native_file_mtime(args).unwrap();
        
        match result {
            Value::Number(mtime) => assert!(mtime > 0.0),
            _ => panic!("Expected number result"),
        }
    }

    #[test]
    fn test_file_mtime_nonexistent() {
        let args = vec![Value::Text(Rc::from("/nonexistent/file.txt"))];
        let result = native_file_mtime(args);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_simple() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content\r\nwith\r\nmixed\nline endings").unwrap();

        let args = vec![Value::Text(Rc::from(file_path.to_string_lossy().as_ref()))];
        let result = native_read_file_simple(args).unwrap();
        
        match result {
            Value::Text(content) => assert_eq!(content.as_ref(), "test content\nwith\nmixed\nline endings"),
            _ => panic!("Expected text result"),
        }
    }

    #[test]
    fn test_write_file_simple() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("output.txt");
        
        let args = vec![
            Value::Text(Rc::from(file_path.to_string_lossy().as_ref())),
            Value::Text(Rc::from("test content\r\nwith\r\nmixed\nline endings")),
        ];
        
        let result = native_write_file_simple(args).unwrap();
        
        match result {
            Value::Text(_) => {
                let written_content = fs::read_to_string(&file_path).unwrap();
                assert_eq!(written_content, "test content\nwith\nmixed\nline endings");
            },
            _ => panic!("Expected text result"),
        }
    }

    #[test]
    fn test_walk_dir() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.path().join("file2.md"), "content2").unwrap();
        fs::write(subdir.join("file3.rs"), "content3").unwrap();

        let args = vec![Value::Text(Rc::from(temp_dir.path().to_string_lossy().as_ref()))];
        let result = native_walk_dir(args).unwrap();
        
        match result {
            Value::List(files) => {
                assert_eq!(files.len(), 3);
                let file_paths: Vec<String> = files.iter()
                    .map(|f| match f {
                        Value::Text(path) => path.as_ref().to_string(),
                        _ => panic!("Expected text in file list"),
                    })
                    .collect();
                
                assert!(file_paths.iter().any(|p| p.ends_with("file1.txt")));
                assert!(file_paths.iter().any(|p| p.ends_with("file2.md")));
                assert!(file_paths.iter().any(|p| p.ends_with("file3.rs")));
            },
            _ => panic!("Expected list result"),
        }
    }

    #[test]
    fn test_walk_dir_with_depth_limit() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        
        fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(subdir.join("file2.txt"), "content2").unwrap();

        let args = vec![
            Value::Text(Rc::from(temp_dir.path().to_string_lossy().as_ref())),
            Value::Number(0.0),
        ];
        let result = native_walk_dir(args).unwrap();
        
        match result {
            Value::List(files) => {
                assert_eq!(files.len(), 1);
                let file_path = match &files[0] {
                    Value::Text(path) => path.as_ref(),
                    _ => panic!("Expected text in file list"),
                };
                assert!(file_path.ends_with("file1.txt"));
            },
            _ => panic!("Expected list result"),
        }
    }

    #[test]
    fn test_glob_files_star_extension() {
        let files = vec![
            Value::Text(Rc::from("file1.md")),
            Value::Text(Rc::from("file2.txt")),
            Value::Text(Rc::from("file3.md")),
            Value::Text(Rc::from("subdir/file4.md")),
        ];
        
        let args = vec![
            Value::Text(Rc::from("*.md")),
            Value::List(files),
        ];
        
        let result = native_glob_files(args).unwrap();
        
        match result {
            Value::List(matched) => {
                assert_eq!(matched.len(), 3);
                let paths: Vec<String> = matched.iter()
                    .map(|f| match f {
                        Value::Text(path) => path.as_ref().to_string(),
                        _ => panic!("Expected text in matched list"),
                    })
                    .collect();
                
                assert!(paths.contains(&"file1.md".to_string()));
                assert!(paths.contains(&"file3.md".to_string()));
                assert!(paths.contains(&"subdir/file4.md".to_string()));
            },
            _ => panic!("Expected list result"),
        }
    }

    #[test]
    fn test_glob_files_recursive_pattern() {
        let files = vec![
            Value::Text(Rc::from("file1.md")),
            Value::Text(Rc::from("docs/file2.md")),
            Value::Text(Rc::from("src/lib.rs")),
            Value::Text(Rc::from("src/main.rs")),
        ];
        
        let args = vec![
            Value::Text(Rc::from("**/*.rs")),
            Value::List(files),
        ];
        
        let result = native_glob_files(args).unwrap();
        
        match result {
            Value::List(matched) => {
                assert_eq!(matched.len(), 2);
                let paths: Vec<String> = matched.iter()
                    .map(|f| match f {
                        Value::Text(path) => path.as_ref().to_string(),
                        _ => panic!("Expected text in matched list"),
                    })
                    .collect();
                
                assert!(paths.contains(&"src/lib.rs".to_string()));
                assert!(paths.contains(&"src/main.rs".to_string()));
            },
            _ => panic!("Expected list result"),
        }
    }

    #[test]
    fn test_parse_cli_args_help() {
        let args = vec![
            Value::Text(Rc::from("--help")),
        ];
        
        let result = native_parse_cli_args(vec![Value::List(args)]);
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.exit_code, 2);
        assert!(error.message.contains("usage: combiner.wfl"));
    }

    #[test]
    fn test_parse_cli_args_flags() {
        let args = vec![
            Value::Text(Rc::from("--output")),
            Value::Text(Rc::from("test.md")),
            Value::Text(Rc::from("--type")),
            Value::Text(Rc::from("docs")),
            Value::Text(Rc::from("--recursive")),
            Value::Text(Rc::from("--no-toc")),
        ];
        
        let result = native_parse_cli_args(vec![Value::List(args)]).unwrap();
        
        match result {
            Value::List(parsed) => {
                assert!(parsed.len() >= 8); // At least 4 key-value pairs
                
                let mut found_output = false;
                let mut found_type = false;
                let mut found_recursive = false;
                let mut found_no_toc = false;
                
                for i in (0..parsed.len()).step_by(2) {
                    if let (Value::Text(key), value) = (&parsed[i], &parsed[i + 1]) {
                        match key.as_ref() {
                            "output" => {
                                found_output = true;
                                if let Value::Text(val) = value {
                                    assert_eq!(val.as_ref(), "test.md");
                                }
                            },
                            "type" => {
                                found_type = true;
                                if let Value::Text(val) = value {
                                    assert_eq!(val.as_ref(), "docs");
                                }
                            },
                            "recursive" => {
                                found_recursive = true;
                                if let Value::Bool(val) = value {
                                    assert!(*val);
                                }
                            },
                            "no-toc" => {
                                found_no_toc = true;
                                if let Value::Bool(val) = value {
                                    assert!(*val);
                                }
                            },
                            _ => {},
                        }
                    }
                }
                
                assert!(found_output);
                assert!(found_type);
                assert!(found_recursive);
                assert!(found_no_toc);
            },
            _ => panic!("Expected list result"),
        }
    }

    #[test]
    fn test_parse_cli_args_short_flags() {
        let args = vec![
            Value::Text(Rc::from("-o")),
            Value::Text(Rc::from("output.md")),
            Value::Text(Rc::from("-r")),
        ];
        
        let result = native_parse_cli_args(vec![Value::List(args)]).unwrap();
        
        match result {
            Value::List(parsed) => {
                let mut found_output = false;
                let mut found_recursive = false;
                
                for i in (0..parsed.len()).step_by(2) {
                    if let (Value::Text(key), value) = (&parsed[i], &parsed[i + 1]) {
                        match key.as_ref() {
                            "output" => {
                                found_output = true;
                                if let Value::Text(val) = value {
                                    assert_eq!(val.as_ref(), "output.md");
                                }
                            },
                            "recursive" => {
                                found_recursive = true;
                                if let Value::Bool(val) = value {
                                    assert!(*val);
                                }
                            },
                            _ => {},
                        }
                    }
                }
                
                assert!(found_output);
                assert!(found_recursive);
            },
            _ => panic!("Expected list result"),
        }
    }

    #[test]
    fn test_help_text_format() {
        let help = get_help_text();
        
        assert!(help.contains("usage: combiner.wfl"));
        assert!(help.contains("WFL File Combiner"));
        assert!(help.contains("-h, --help"));
        assert!(help.contains("-o OUTPUT, --output OUTPUT"));
        assert!(help.contains("--type {docs,src,both}"));
        assert!(help.contains("-r, --recursive"));
        assert!(help.contains("--no-toc"));
        
        for line in help.lines() {
            assert!(line.len() <= 80, "Line too long: {}", line);
        }
    }

    #[test]
    fn test_help_text_golden() {
        let expected_help = "usage: combiner.wfl [-h] [-o OUTPUT] [-i INPUT] [--type {docs,src,both}]\n                    [-r] [--no-toc] [-s SORT] [-l HEADER_LEVEL]\n                    [-p SEPARATOR] [-a] [--no-txt]\n\nWFL File Combiner - Combine multiple files into markdown and text files\n\noptions:\n  -h, --help            show this help message and exit\n  -o OUTPUT, --output OUTPUT\n                        Path and filename for the combined output file\n  -i INPUT, --input INPUT\n                        Directory containing files (default: based on --type)\n  --type {docs,src,both}\n                        Type of files to process: 'docs' for markdown files,\n                        'src' for Rust files, or 'both' to process both types\n  -r, --recursive       Search subdirectories for files (always enabled for\n                        src)\n  --no-toc              Disable table of contents (enabled by default)\n  -s SORT, --sort SORT  Sort files by: 'alpha', 'time', or comma-separated\n                        list for custom order\n  -l HEADER_LEVEL, --header-level HEADER_LEVEL\n                        Starting level for file headers (default: 1)\n  -p SEPARATOR, --separator SEPARATOR\n                        Custom separator between files (default: horizontal\n                        rule)\n  -a, --all-files       Include all .md files in Docs, not just those with\n                        'wfl-' in the name\n  --no-txt              Disable output to .txt format (by default outputs to\n                        both .md and .txt)";
        
        let actual_help = get_help_text();
        assert_eq!(actual_help, expected_help, "Help text must match exactly to ensure byte-for-byte parity with Python version");
    }

    #[test]
    fn test_toc_anchor_generation() {
        fn generate_toc_anchor(title: &str) -> String {
            title.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join("-")
        }

        assert_eq!(generate_toc_anchor("Hello World"), "hello-world");
        assert_eq!(generate_toc_anchor("User's Guide"), "users-guide");
        assert_eq!(generate_toc_anchor("API Reference!"), "api-reference");
        assert_eq!(generate_toc_anchor("File I/O Operations"), "file-io-operations");
        assert_eq!(generate_toc_anchor("Multi-Word Title"), "multi-word-title");
        assert_eq!(generate_toc_anchor("Special@#$%Characters"), "specialcharacters");
        assert_eq!(generate_toc_anchor("Numbers 123 Test"), "numbers-123-test");
        assert_eq!(generate_toc_anchor("   Extra   Spaces   "), "extra-spaces");
    }
}
