mod analyzer;
mod thread_pool;

use analyzer::{analyze_file, FileAnalysis};
use thread_pool::ThreadPool;

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;
use std::{io, thread};

#[derive(Debug, Clone, Copy)]
enum Status {
    Queued,
    Running,
    Done,
    Error,
    Canceled,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <num_threads> <dir1> [dir2 dir3 ...]", args[0]);
        std::process::exit(1);
    }

    let num_threads: usize = args[1].parse().expect("num_threads must be a number");
    let dirs: Vec<String> = args[2..].to_vec();

    let mut files: Vec<PathBuf> = Vec::new();
    for d in &dirs {
        let dir_path = Path::new(d);
        if dir_path.is_dir() {
            collect_files(dir_path, &mut files);
        } else {
            eprintln!("Warning: {} is not a directory, skipping.", d);
        }
    }

    if files.is_empty() {
        eprintln!("No files found to process.");
        return;
    }

    println!("Found {} files to process", files.len());
    println!("Using {} worker threads", num_threads);
    println!("Press Enter to cancel...");

    let results: Arc<Mutex<Vec<FileAnalysis>>> = Arc::new(Mutex::new(Vec::new()));
    let cancel_flag = Arc::new(AtomicBool::new(false));

   
    let status_map: Arc<Mutex<HashMap<String, Status>>> = Arc::new(Mutex::new(HashMap::new()));

    
    {
        let mut sm = status_map.lock().unwrap();
        for p in &files {
            sm.insert(p.display().to_string(), Status::Queued);
        }
    }

    
    let cancel_for_input = Arc::clone(&cancel_flag);
    thread::spawn(move || {
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);
        cancel_for_input.store(true, Ordering::SeqCst);
        eprintln!("Cancellation requested.");
    });

    let mut pool = ThreadPool::new(num_threads);

    
    if let Ok(val) = env::var("RESIZE_TO") {
        if let Ok(n) = val.parse::<usize>() {
            pool.resize(n);
            eprintln!("Resized thread pool to {}", n);
        }
    }

    let start_all = Instant::now();
    let total_files = files.len();

    
    for path in files {
        if cancel_flag.load(Ordering::SeqCst) {
            let mut sm = status_map.lock().unwrap();
            for (_k, v) in sm.iter_mut() {
                if matches!(*v, Status::Queued) {
                    *v = Status::Canceled;
                }
            }
            break;
        }

        let results = Arc::clone(&results);
        let cancel_flag = Arc::clone(&cancel_flag);
        let status_map = Arc::clone(&status_map);
        let full_path = path.display().to_string();

        pool.execute(move || {
            if cancel_flag.load(Ordering::SeqCst) {
                let mut sm = status_map.lock().unwrap();
                sm.insert(full_path.clone(), Status::Canceled);
                return;
            }

            {
                let mut sm = status_map.lock().unwrap();
                sm.insert(full_path.clone(), Status::Running);
            }

            let analysis = analyze_file(&path);
            let is_error = !analysis.errors.is_empty();

            {
                let mut r = results.lock().unwrap();
                r.push(analysis.clone());
            }

            {
                let mut sm = status_map.lock().unwrap();
                sm.insert(full_path.clone(), if is_error { Status::Error } else { Status::Done });
            }

            
            let done_count = {
                let sm = status_map.lock().unwrap();
                sm.values().filter(|s| matches!(s, Status::Done | Status::Error | Status::Canceled)).count()
            };

            println!(
                "[{}/{}] {:?} ({}) in {:?}  errors:{}",
                done_count,
                total_files,
                analysis.filename,
                analysis.full_path,
                analysis.processing_time,
                analysis.errors.len()
            );
        });
    }

    pool.shutdown();
    let total_time = start_all.elapsed();

    
    let analyses = results.lock().unwrap();
    let sm = status_map.lock().unwrap();

    let mut total_words = 0usize;
    let mut total_lines = 0usize;
    let mut total_size = 0u64;
    let mut total_errors = 0usize;

    for fa in analyses.iter() {
        total_words += fa.stats.word_count;
        total_lines += fa.stats.line_count;
        total_size += fa.stats.size_bytes;
        total_errors += fa.errors.len();
    }

    let done = sm.values().filter(|s| matches!(s, Status::Done)).count();
    let err = sm.values().filter(|s| matches!(s, Status::Error)).count();
    let canceled = sm.values().filter(|s| matches!(s, Status::Canceled)).count();

    println!("\n=== SUMMARY ===");
    println!("Done: {done}, Error: {err}, Canceled: {canceled}");
    println!("Files analyzed records: {}", analyses.len());
    println!("Total wall-clock time: {:?}", total_time);
    println!("Total words: {total_words}");
    println!("Total lines: {total_lines}");
    println!("Total size (bytes): {total_size}");
    println!("Total errors: {total_errors}");

    if total_errors > 0 {
        println!("\n=== ERRORS (with context) ===");
        for fa in analyses.iter() {
            if !fa.errors.is_empty() {
                println!("File: {}", fa.full_path);
                for e in &fa.errors {
                    println!("  - {:?}", e);
                }
            }
        }
    }
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_files(&path, out);
                } else if path.is_file() {
                    out.push(path);
                }
            }
        }
        Err(e) => {
            eprintln!("read_dir error on {}: {}", dir.display(), e);
        }
    }
}
