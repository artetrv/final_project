use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use final_project::analyzer::analyze_file; 

#[test]
fn integration_analyze_real_file() {
    let mut path = std::env::temp_dir();
    path.push("integration_book.txt");

    let mut f = File::create(&path).unwrap();
    writeln!(f, "hello world").unwrap();
    writeln!(f, "another line").unwrap();

    let result = analyze_file(&path);
    assert_eq!(result.stats.line_count, 2);
    assert_eq!(result.stats.word_count, 4);
    assert!(result.errors.is_empty());
}

#[test]
fn error_handling_missing_file() {
    let mut path = PathBuf::from(std::env::temp_dir());
    path.push("this_file_should_not_exist_12345.txt");

    let result = analyze_file(&path);
    assert!(!result.errors.is_empty());
}
