use clap::Parser;
use dialoguer::Confirm;
use phf::{Set, phf_set};
use rs_cleaner::Cli;
use std::error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use walkdir::{DirEntry, WalkDir};
const SECONDS_IN_DAY: u64 = 24 * 60 * 60;

static PROJECT_TARGETS: Set<&'static str> = phf_set! {
    "Cargo.toml",
    "package.json"
};

static REMOVE_TARGETS: Set<&'static str> = phf_set! {
    "target",
    "node_modules"
};

#[derive(Debug, Default)]
struct CollectResults {
    paths: Vec<PathBuf>,
    errors: Vec<CollectedError>,
}

#[derive(Debug)]
struct CollectedError {
    path: Option<PathBuf>,
    kind: PathCollectionError,
}

#[derive(Debug)]
enum PathCollectionError {
    WalkDir(walkdir::Error),
    Io(std::io::Error),
}

impl CollectedError {
    fn walkdir(path: Option<PathBuf>, err: walkdir::Error) -> Self {
        Self {
            path,
            kind: PathCollectionError::WalkDir(err),
        }
    }

    fn io(path: Option<PathBuf>, err: std::io::Error) -> Self {
        Self {
            path,
            kind: PathCollectionError::Io(err),
        }
    }
}

fn collect_projects(dir: &Path, depth: usize, days: Option<u64>) -> CollectResults {
    let mut results = CollectResults::default();
    let threshold = days.map(|days| SystemTime::now() - Duration::from_secs(days * SECONDS_IN_DAY));

    let mut walker = WalkDir::new(dir)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|entry| !is_dir_in_target_to_remove(entry));

    while let Some(entry) = walker.next() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                results.errors.push(CollectedError::walkdir(
                    err.path().map(|path| path.to_path_buf()),
                    err,
                ));
                continue;
            }
        };

        if !is_project_in_target(&entry) {
            continue;
        }

        let Some(project_path) = entry.path().parent() else {
            continue;
        };

        let (youngest, errors) = find_youngest_file(project_path);
        let should_add = match threshold {
            Some(threshold) => {
                youngest.is_some_and(|modified| is_older_than_threshold(modified, threshold))
            }
            None => true,
        };

        results.errors.extend(errors);

        if should_add {
            results.paths.push(project_path.to_path_buf());
        }
    }

    results
}

fn find_target_to_remove(project_dir: &Path) -> CollectResults {
    let mut results = CollectResults::default();

    let entries = match fs::read_dir(project_dir) {
        Ok(entries) => entries,
        Err(err) => {
            results
                .errors
                .push(CollectedError::io(Some(project_dir.to_path_buf()), err));
            return results;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                results
                    .errors
                    .push(CollectedError::io(Some(project_dir.to_path_buf()), err));
                continue;
            }
        };

        if let Some(name) = entry.file_name().to_str()
            && REMOVE_TARGETS.contains(name)
        {
            results.paths.push(entry.path());
        }
    }

    results
}

fn is_project_in_target(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| PROJECT_TARGETS.contains(name))
}

fn is_dir_in_target_to_remove(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| REMOVE_TARGETS.contains(name))
}

fn find_youngest_file(path: &Path) -> (Option<SystemTime>, Vec<CollectedError>) {
    let mut errors = Vec::new();
    let mut youngest: Option<SystemTime> = None;

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|entry| !is_dir_in_target_to_remove(entry))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                errors.push(CollectedError::walkdir(
                    err.path().map(|entry_path| entry_path.to_path_buf()),
                    err,
                ));
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let modified = match fs::metadata(entry.path()).and_then(|metadata| metadata.modified()) {
            Ok(time) => time,
            Err(err) => {
                errors.push(CollectedError::io(Some(entry.path().to_path_buf()), err));
                continue;
            }
        };

        if youngest.is_none_or(|current_time| modified > current_time) {
            youngest = Some(modified);
        }
    }

    (youngest, errors)
}

fn is_older_than_threshold(modified: SystemTime, threshold: SystemTime) -> bool {
    modified < threshold
}

fn calculate_size(paths: &[PathBuf]) -> u64 {
    let mut size = 0;

    for path in paths {
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file()
                && let Ok(metadata) = entry.metadata()
            {
                size += metadata.len();
            }
        }
    }

    size
}

fn format_size(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = MB * 1024.0;

    let bytes = bytes as f64;

    if bytes >= GB {
        format!("{bytes:.2} GB", bytes = bytes / GB)
    } else {
        format!("{bytes:.2} MB", bytes = bytes / MB)
    }
}

fn remove_dirs(paths: &[PathBuf]) -> Result<(), Vec<CollectedError>> {
    let mut errors = Vec::new();

    for path in paths {
        if let Err(err) = fs::remove_dir_all(path) {
            errors.push(CollectedError::io(Some(path.clone()), err));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn format_errors() {
    todo!()
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let args = Cli::parse();
    let root = match args.path {
        Some(path) => path,
        None => std::env::current_dir()?,
    };
    let mut errors = Vec::new();

    let now = Instant::now();
    let project_results = collect_projects(&root, args.depth, args.older_than);
    let mut paths_to_remove = Vec::new();
    errors.extend(project_results.errors);
    for project_path in &project_results.paths {
        let target_results = find_target_to_remove(project_path);
        paths_to_remove.extend(target_results.paths);
        errors.extend(target_results.errors);
    }

    let size = calculate_size(&paths_to_remove);
    let elapsed = now.elapsed();

    println!("Found {} directories to remove", paths_to_remove.len());
    println!("Total size: {}", format_size(size));
    println!("Elapsed: {:.2} ms", elapsed.as_secs_f64() * 1000.0);

    for path in &paths_to_remove {
        println!("{}", path.display());
    }

    if !args.auto_accept {
        println!("Do you want to proceed with deletion?");
        let proceed = Confirm::new()
            .with_prompt("Continue?")
            .default(true)
            .interact()
            .unwrap();

        if proceed {
            // here we deleting
            println!("deleted all the file");
        }
    } else {
        println!("deleted all the file");
    }

    if !args.verbose {
        println!("Found {} errors", errors.iter().count())
    } else {
        println!("Errors found: ");
        for err in errors {
            //format error
            match err.kind {
                PathCollectionError::WalkDir(err) => println!("walkdir error"),
                PathCollectionError::Io(err) => println!("Io error"),
            }
        }
    }

    Ok(())
}
