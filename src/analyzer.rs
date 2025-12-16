use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum ProcessingError {
    IoError(String),
    Other(String),
}

#[derive(Debug, Default, Clone)]
pub struct FileStats {
    pub word_count: usize,
    pub line_count: usize,
    pub char_frequencies: HashMap<char, usize>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub filename: String, 
    pub full_path: String, 
    pub stats: FileStats,
    pub errors: Vec<ProcessingError>,
    pub processing_time: Duration,
}

pub fn analyze_file(path: &Path) -> FileAnalysis {
    let start = Instant::now();
    let mut errors = Vec::new();
    let mut stats = FileStats::default();

    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let full_path = path.display().to_string();

    match fs::metadata(path) {
        Ok(meta) => stats.size_bytes = meta.len(),
        Err(e) => errors.push(ProcessingError::IoError(format!(
            "metadata error on {}: {}",
            full_path, e
        ))),
    }

    match fs::File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        stats.line_count += 1;
                        stats.word_count += line.split_whitespace().count();
                        for ch in line.chars() {
                            *stats.char_frequencies.entry(ch).or_insert(0) += 1;
                        }
                        
                    }
                    Err(e) => {
                        errors.push(ProcessingError::IoError(format!(
                            "line read error on {}: {}",
                            full_path, e
                        )));
                        break;
                    }
                }
            }
        }
        Err(e) => errors.push(ProcessingError::IoError(format!(
            "open error on {}: {}",
            full_path, e
        ))),
    }

    FileAnalysis {
        filename,
        full_path,
        stats,
        errors,
        processing_time: start.elapsed(),
    }
}
