use std::time::Instant;
use std::path::Path;
use final_project::analyzer::analyze_file;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: bench <dir>");
    let mut files = Vec::new();
    collect_files(Path::new(&dir), &mut files);

    let start = Instant::now();
    let mut total = 0usize;

    for f in files.iter().take(100) {
        let a = analyze_file(f);
        total += a.stats.word_count;
    }

    println!("Bench: processed 100 files in {:?}. total_words={}", start.elapsed(), total);
}

fn collect_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() { collect_files(&p, out); }
            else if p.is_file() { out.push(p); }
        }
    }
}
