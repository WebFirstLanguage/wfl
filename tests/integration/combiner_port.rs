use std::fs;
use std::path::Path;
use tempfile::tempdir;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[tokio::test]
async fn test_combiner_integration() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();
    
    let file1_path = test_dir.join("test1.md");
    let file2_path = test_dir.join("test2.md");
    let file3_path = test_dir.join("wfl-test3.md");
    
    fs::write(&file1_path, "# Test File 1\n\nContent of test file 1.").expect("Failed to write test file 1");
    fs::write(&file2_path, "# Test File 2\n\nContent of test file 2.").expect("Failed to write test file 2");
    fs::write(&file3_path, "# WFL Test File 3\n\nContent of WFL test file 3.").expect("Failed to write test file 3");
    
    let output_path = test_dir.join("combined.md");
    
    let wfl_script = format!(r#"
    store cli_args as args()
    store input_dir as "{}"
    store output_file as "{}"
    
    store files as dirlist(input_dir, false, "*.md")
    store file_count as length(files)
    display "Found " plus file_count plus " files"
    
    store sorted_files as sort(files)
    
    store content as ""
    store count as 0
    
    for each file in sorted_files:
        change count to count plus 1
        store base_name as basename(file)
        store dir_name as dirname(file)
        store mtime as filemtime(file)
        
        check if count is greater than 1:
            change content to content plus "\n---\n"
        end check
        
        change content to content plus "File: " plus base_name plus "\n"
        change content to content plus "Directory: " plus dir_name plus "\n"
        change content to content plus "Modified: " plus mtime plus "\n\n"
    end for
    
    store final_output as pathjoin(dirname(output_file), basename(output_file))
    display "Writing to: " plus final_output
    "#, 
    test_dir.to_str().unwrap().replace('\\', "\\\\"),
    output_path.to_str().unwrap().replace('\\', "\\\\"));

    let tokens = lex_wfl_with_positions(&wfl_script);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse WFL script");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "WFL script execution failed: {:?}", result);
    
}

#[tokio::test]
async fn test_args_function_integration() {
    let wfl_script = r#"
    store cli_args as args()
    store arg_count as length(cli_args)
    display "Argument count: " plus arg_count
    "#;

    let tokens = lex_wfl_with_positions(wfl_script);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse WFL script");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "args() integration test failed: {:?}", result);
}

#[tokio::test]
async fn test_filesystem_functions_integration() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "test content").expect("Failed to write test file");

    let wfl_script = format!(r#"
    store test_dir as "{}"
    store test_file as "{}"
    
    store files as dirlist(test_dir, false, "*.txt")
    store file_count as length(files)
    
    check if file_count is greater than 0:
        store first_file as index(files, 0)
        store mtime as filemtime(first_file)
        store base as basename(first_file)
        store dir as dirname(first_file)
        
        display "Found file: " plus base
        display "In directory: " plus dir
        display "Modified time: " plus mtime
    end check
    "#, 
    temp_dir.path().to_str().unwrap().replace('\\', "\\\\"),
    test_file.to_str().unwrap().replace('\\', "\\\\"));

    let tokens = lex_wfl_with_positions(&wfl_script);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse WFL script");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Filesystem functions integration test failed: {:?}", result);
}

#[tokio::test]
async fn test_sort_function_integration() {
    let wfl_script = r#"
    create list as numbers
    push with numbers and 3
    push with numbers and 1
    push with numbers and 4
    push with numbers and 2
    
    store sorted_asc as sort(numbers, null, false)
    store sorted_desc as sort(numbers, null, true)
    
    store first_asc as index(sorted_asc, 0)
    store first_desc as index(sorted_desc, 0)
    
    display "First ascending: " plus first_asc
    display "First descending: " plus first_desc
    "#;

    let tokens = lex_wfl_with_positions(wfl_script);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse WFL script");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Sort function integration test failed: {:?}", result);
}

#[tokio::test]
async fn test_path_functions_integration() {
    let wfl_script = r#"
    store full_path as "/home/user/documents/file.txt"
    store base as basename(full_path)
    store dir as dirname(full_path)
    store joined as pathjoin("/home", "user", "file.txt")
    
    display "Basename: " plus base
    display "Dirname: " plus dir
    display "Joined path: " plus joined
    "#;

    let tokens = lex_wfl_with_positions(wfl_script);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse WFL script");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Path functions integration test failed: {:?}", result);
}

#[tokio::test]
async fn test_all_new_functions_comprehensive() {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();
    
    let md_file = test_dir.join("test.md");
    let txt_file = test_dir.join("test.txt");
    let rs_file = test_dir.join("test.rs");
    
    fs::write(&md_file, "# Markdown content").expect("Failed to write md file");
    fs::write(&txt_file, "Text content").expect("Failed to write txt file");
    fs::write(&rs_file, "fn main() {}").expect("Failed to write rs file");

    let wfl_script = format!(r#"
    store test_dir as "{}"
    store args_list as args()
    
    store all_files as dirlist(test_dir, false, "*")
    store md_files as dirlist(test_dir, false, "*.md")
    store sorted_files as sort(all_files)
    
    store file_info as ""
    for each file in sorted_files:
        store base as basename(file)
        store dir as dirname(file)
        store mtime as filemtime(file)
        store info_line as pathjoin("File:", base)
        change file_info to file_info plus info_line plus "\n"
    end for
    
    display "Processed " plus length(sorted_files) plus " files"
    display "Found " plus length(md_files) plus " markdown files"
    "#, test_dir.to_str().unwrap().replace('\\', "\\\\"));

    let tokens = lex_wfl_with_positions(&wfl_script);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Failed to parse WFL script");

    let mut interpreter = Interpreter::default();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Comprehensive integration test failed: {:?}", result);
}
