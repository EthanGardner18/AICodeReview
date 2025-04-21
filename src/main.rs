use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FileInfo {
    path: String,
    modified: u64,
}

// The state for all directories is stored here.
type DirectoryState = HashMap<String, Vec<FileInfo>>;

/// Main function that orchestrates the program flow
fn main() -> io::Result<()> {
    dotenv().ok();
    let dir_path = env::var("DIRECTORY").expect("DIRECTORY env variable not set");
    let canonical_dir = normalize_path(&dir_path)?;
    
    let path = Path::new(&canonical_dir);
    if !path.is_dir() {
        eprintln!("The path specified is not a directory: {}", dir_path);
        std::process::exit(1);
    }

    // Load the state file
    let state_file = "state.json";
    let mut state_map = load_state_map(state_file)?;
    
    // Process directory and handle nested directories
    if let Some(result) = process_directory(&canonical_dir, path, &mut state_map)? {
        display_changes(&result);
        
        // If there are added or modified files, copy them to the temp directory
        if !result.added.is_empty() || !result.modified.is_empty() {
            copy_files_to_temp(&result)?;
        }
        
        // Clean the temp directory afterward (you can comment this out if you want to keep the files)
        //clean_temp_directory()?;
    }
    
    // Save updated state
    save_state_map(&state_map, state_file)?;
    
    Ok(())
}

struct ChangeResults {
    added: Vec<FileInfo>,
    deleted: Vec<FileInfo>,
    modified: Vec<(FileInfo, FileInfo)>,
}

/// Processes a directory, determining changes and handling nested directory relationships
fn process_directory(canonical_dir: &str, path: &Path, state_map: &mut DirectoryState) -> io::Result<Option<ChangeResults>> {
    // Always collect current state of code files in this directory
    let new_state = collect_code_files(path)?;
    
    // Check if this directory is a subdirectory of another tracked directory
    let is_subdir = is_subdirectory_of_tracked_dir(canonical_dir, state_map);
    
    // Get all files from subdirectories that would be consolidated
    let mut previously_tracked_files = Vec::new();
    let subdirs: Vec<String> = state_map.keys()
        .filter(|existing_dir| **existing_dir != canonical_dir && existing_dir.starts_with(canonical_dir))
        .cloned()
        .collect();
    
    // Collect all files from subdirectories
    for subdir in &subdirs {
        if let Some(files) = state_map.get(subdir) {
            previously_tracked_files.extend(files.clone());
        }
    }
    
    // Get previous state for this exact directory
    let old_state = if state_map.contains_key(canonical_dir) {
        state_map.get(canonical_dir).unwrap().clone()
    } else {
        Vec::new()
    };
    
    // Determine what state to use for comparison
    let comparison_state = if is_subdir {
        // For subdirectories, find the relevant files in the parent directory
        let parent_dir = find_parent_directory(canonical_dir, state_map);
        if let Some(parent) = parent_dir {
            if let Some(parent_files) = state_map.get(&parent) {
                // Filter parent's files to only include those in this subdirectory
                parent_files.iter()
                    .filter(|fi| fi.path.starts_with(canonical_dir))
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            old_state
        }
    } else {
        // For parent directories, combine with previously tracked files
        let mut combined = old_state;
        combined.extend(previously_tracked_files);
        combined
    };
    
    // Compare states and identify changes
    let changes = compare_states(comparison_state, &new_state);
    
    // Only update the state map if this isn't a subdirectory of another tracked dir
    if !is_subdir {
        // Remove any previously tracked subdirectories
        for subdir in subdirs {
            state_map.remove(&subdir);
        }
        
        // Update or insert this directory's state
        state_map.remove(canonical_dir); // Remove first to avoid borrowing issues
        state_map.insert(canonical_dir.to_string(), new_state);
    }
    
    Ok(Some(changes))
}

/// Finds the most specific parent directory that contains the given directory
fn find_parent_directory(dir: &str, state_map: &DirectoryState) -> Option<String> {
    let mut parent = None;
    let mut parent_len = 0;
    
    for existing_dir in state_map.keys() {
        if dir != existing_dir && dir.starts_with(existing_dir) {
            // Find the longest matching parent (most specific)
            if existing_dir.len() > parent_len {
                parent = Some(existing_dir.clone());
                parent_len = existing_dir.len();
            }
        }
    }
    
    parent
}

/// Checks if a directory is a subdirectory of any tracked directory
fn is_subdirectory_of_tracked_dir(dir: &str, state_map: &DirectoryState) -> bool {
    for existing_dir in state_map.keys() {
        if dir != existing_dir && dir.starts_with(existing_dir) {
            return true;
        }
    }
    false
}

/// Removes subdirectories from tracking when consolidating to a parent directory
fn remove_tracked_subdirectories(dir: &str, state_map: &mut DirectoryState) {
    let subdirs: Vec<String> = state_map.keys()
        .filter(|existing_dir| **existing_dir != dir && existing_dir.starts_with(dir))
        .cloned()
        .collect();
    
    for subdir in subdirs {
        state_map.remove(&subdir);
    }
}

/// Loads the state map from a JSON file
fn load_state_map(state_file: &str) -> io::Result<DirectoryState> {
    if Path::new(state_file).exists() {
        let data = fs::read_to_string(state_file)?;
        Ok(serde_json::from_str(&data).unwrap_or_default())
    } else {
        Ok(HashMap::new())
    }
}

/// Saves the state map to a JSON file
fn save_state_map(state_map: &DirectoryState, state_file: &str) -> io::Result<()> {
    let json = serde_json::to_string_pretty(state_map)?;
    fs::write(state_file, json)
}

/// Compares old and new file states to determine changes
fn compare_states(old_state: Vec<FileInfo>, new_state: &[FileInfo]) -> ChangeResults {
    // Convert states to maps for easier comparison
    let old_map: HashMap<String, FileInfo> = old_state
        .into_iter()
        .map(|fi| (fi.path.clone(), fi))
        .collect();
    
    let new_map: HashMap<String, FileInfo> = new_state
        .iter()
        .map(|fi| (fi.path.clone(), fi.clone()))
        .collect();

    // Find added files
    let added = new_map.values()
        .filter(|fi| !old_map.contains_key(&fi.path))
        .cloned()
        .collect();

    // Find deleted files
    let deleted = old_map.values()
        .filter(|fi| !new_map.contains_key(&fi.path))
        .cloned()
        .collect();

    // Find modified files
    let modified = new_map.iter()
        .filter_map(|(path, new_fi)| {
            if let Some(old_fi) = old_map.get(path) {
                if new_fi.modified != old_fi.modified {
                    Some((old_fi.clone(), new_fi.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    ChangeResults { added, deleted, modified }
}

/// Displays changes to the console
fn display_changes(changes: &ChangeResults) {
    if changes.added.is_empty() && changes.deleted.is_empty() && changes.modified.is_empty() {
        println!("No changes detected.");
        return;
    }

    if !changes.added.is_empty() {
        println!("Added files:");
        for fi in &changes.added {
            println!("  {}", fi.path);
        }
    }
    
    if !changes.deleted.is_empty() {
        println!("Deleted files:");
        for fi in &changes.deleted {
            println!("  {}", fi.path);
        }
    }
    
    if !changes.modified.is_empty() {
        println!("Modified files:");
        for (_, new_fi) in &changes.modified {
            println!("  {}", new_fi.path);
        }
    }
}

/// Normalizes a path to a consistent format
fn normalize_path<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let canonical = fs::canonicalize(path)?;
    
    // Convert to string and fix potential double slashes
    let path_str = canonical.to_string_lossy().to_string();
    
    // On Windows, this will have backslashes which we convert to forward slashes
    let normalized = path_str.replace('\\', "/");
    
    // Remove potential double slashes
    let normalized = normalized.replace("//", "/");
    
    Ok(normalized)
}

/// Recursively collects information on code files from a directory
fn collect_code_files(dir: &Path) -> io::Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_code_files(&path)?);
        } else if path.is_file() && is_code_file(&path) {
            if let Ok(fi) = get_file_info(&path) {
                files.push(fi);
            }
        }
    }
    Ok(files)
}

/// Gets file information including path and modified timestamp
fn get_file_info(path: &Path) -> io::Result<FileInfo> {
    let metadata = fs::metadata(path)?;
    let modified_time = metadata.modified()?;
    let duration = modified_time.duration_since(UNIX_EPOCH).unwrap_or_default();
    
    // Convert to string and normalize path
    let path_str = path.to_string_lossy().to_string().replace('\\', "/");
    let normalized_path = path_str.replace("//", "/");
    
    Ok(FileInfo {
        path: normalized_path,
        modified: duration.as_secs(),
    })
}

/// Checks if a file is a code file based on its extension
fn is_code_file(path: &Path) -> bool {
    let code_extensions = [
        "as", "ascx", "asm", "asp", "aspx", "bas", "c", "c++", "cc", "clj", "coffee", "cpp", 
        "cs", "dart", "el", "elm", "erl", "ex", "exs", "f90", "f95", "fs", "fsi", "fsx", 
        "go", "groovy", "h", "h++", "hh", "hpp", "hs", "htm", "html", "hxx", "ino", "java", 
        "jl", "js", "jsx", "kt", "kts", "lisp", "lua", "m", "ml", "mli", "nim", "php", "pl", 
        "pm", "py", "r", "rb", "rs", "sass", "scala", "scm", "scss", "sh", "sml", "sql", 
        "styl", "swift", "tex", "ts", "tsx", "vb", "vbs", "vue", "zsh"
    ];

    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
        code_extensions.contains(&extension)
    } else {
        false
    }
}

/// Copies added and modified files to the temp directory
fn copy_files_to_temp(changes: &ChangeResults) -> io::Result<()> {
    // Create the temp directory if it doesn't exist
    let temp_dir = PathBuf::from("temp");
    if !temp_dir.exists() {
        fs::create_dir(&temp_dir)?;
    }
    
    println!("Copying files to temp directory...");
    
    // Copy added files
    for file_info in &changes.added {
        copy_file_to_temp(&file_info.path, &temp_dir)?;
    }
    
    // Copy modified files
    for (_, file_info) in &changes.modified {
        copy_file_to_temp(&file_info.path, &temp_dir)?;
    }
    
    println!("Files copied to temp directory successfully.");
    Ok(())
}

/// Copies an individual file to the temp directory
fn copy_file_to_temp(file_path: &str, temp_dir: &Path) -> io::Result<()> {
    let source_path = Path::new(file_path);
    
    // Get the filename for the destination
    let file_name = match source_path.file_name() {
        Some(name) => name,
        None => return Err(io::Error::new(ErrorKind::InvalidInput, format!("Invalid file path: {}", file_path))),
    };
    
    let dest_path = temp_dir.join(file_name);
    
    // Copy the file
    
    match fs::copy(source_path, &dest_path) {
        Ok(_) => {
            Ok(())
        },
        Err(e) => {
            eprintln!("  Failed to copy {}: {}", file_path, e);
            Err(e)
        }
    }
    
}

/// Cleans (empties) the temp directory
fn clean_temp_directory() -> io::Result<()> {
    let temp_dir = PathBuf::from("temp");
    
    // If the directory doesn't exist, nothing to clean
    if !temp_dir.exists() {
        return Ok(());
    }
    
    println!("Cleaning temp directory...");
    
    // Read all entries in the temp directory
    for entry in fs::read_dir(&temp_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Remove file or directory
        if path.is_file() {
            fs::remove_file(&path)?;
        } else if path.is_dir() {
            fs::remove_dir_all(&path)?;
        }
    }
    
    println!("Temp directory cleaned successfully.");
    Ok(())
}