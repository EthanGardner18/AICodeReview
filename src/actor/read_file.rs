#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use std::io;
use std::fs;
use std::path::{Path, PathBuf};
use std::cmp::min;

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileData {
    pub path: String,
    pub content: String,
    pub lastFile: String,
}

#[derive(Default)]
pub(crate) struct ReadfileInternalState {}

#[cfg(not(test))]
pub async fn run(
    context: SteadyContext,
    file_data_tx: SteadyTx<FileData>,
    state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    let _cli_args = context.args::<Args>();
    let cmd = into_monitor!(context, [], [file_data_tx]);
    internal_behavior(cmd, file_data_tx, state).await
}

fn scan_directory_for_files(path: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found_files = Vec::new();
    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    found_files.extend(scan_directory_for_files(&entry_path, extensions));
                } else if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        found_files.push(entry_path);
                    }
                }
            }
        } else {
            println!("Failed to read directory '{}'", path.display());
        }
    }
    found_files
}

const MAX_CHUNK_SIZE: usize = 100 * 1024; // 100 KB

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    file_data_tx: SteadyTx<FileData>,
    state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || ReadfileInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {
        let mut file_data_tx = file_data_tx.lock().await;

        let mut input_path = String::new();
        println!("Enter the file path:");
        io::stdin().read_line(&mut input_path).expect("Failed to read line");
        let input_path = input_path.trim().to_string();
        let input_dir = PathBuf::from(&input_path);

        let coding_extensions = [
            "py", "cpp", "hpp", "cc", "cxx", "rs", "c", "js", "jsx", "ts", "tsx", "java", "go",
            "sh", "php", "rb", "kt", "kts", "swift", "pl", "pm", "r", "pas", "f90", "lisp", "cbl",
            "zig",
        ];

        let found_files = scan_directory_for_files(&input_dir, &coding_extensions);
        let total_files = found_files.len();
        let mut processed_files = 0;

        while cmd.is_running(&mut || file_data_tx.mark_closed()) {
            let _clean = await_for_all!(cmd.wait_periodic(Duration::from_millis(1000)));

            for file_path in &found_files {
                processed_files += 1;
                let is_last_file = processed_files == total_files;

                let content = fs::read_to_string(file_path).unwrap_or_else(|_| {
                    eprintln!("Failed to read file: {}", file_path.display());
                    String::new()
                });

                let mut chunk_buf = String::new();
                let mut current_chunk_size = 0;
                let mut line_number = 1;

                for line in content.lines() {
                    let numbered_line = format!("{:>6}: {}\n", line_number, line);
                    let line_bytes_len = numbered_line.as_bytes().len();
                    line_number += 1;

                    if current_chunk_size + line_bytes_len > MAX_CHUNK_SIZE {
                        let data = FileData {
                            path: file_path.display().to_string(),
                            content: chunk_buf.clone(),
                            lastFile: "F".to_string(),
                        };

                        match cmd.try_send(&mut file_data_tx, data) {
                            Ok(()) => {
                                println!("Sent chunk ({} bytes)", chunk_buf.len());
                            }
                            Err(msg) => trace!("Error sending: {:?}", msg),
                        }

                        chunk_buf.clear();
                        current_chunk_size = 0;
                    }

                    chunk_buf.push_str(&numbered_line);
                    current_chunk_size += line_bytes_len;
                }

                if !chunk_buf.is_empty() {
                    let data = FileData {
                        path: file_path.display().to_string(),
                        content: chunk_buf.clone(),
                        lastFile: if is_last_file { "T".to_string() } else { "F".to_string() },
                    };

                    match cmd.try_send(&mut file_data_tx, data) {
                        Ok(()) => {
                            println!(
                                "Sent final chunk ({} bytes) {}",
                                chunk_buf.len(),
                                if is_last_file { "(last chunk of last file)" } else { "" }
                            );

                            if is_last_file {
                                file_data_tx.mark_closed();
                                println!("file_data_tx marked as closed.");
                                println!("All files processed. Exiting actor.");
                                return Ok(());
                            }
                        }
                        Err(msg) => trace!("Error sending: {:?}", msg),
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub async fn run(
    context: SteadyContext,
    file_data_tx: SteadyTx<FileData>,
    state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut cmd = into_monitor!(context, [], [file_data_tx]);
    if let Some(responder) = cmd.sidechannel_responder() {
        let mut file_data_tx = file_data_tx.lock().await;
        while cmd.is_running(&mut || file_data_tx.mark_closed()) {
            let _did_echo = responder.echo_responder(&mut cmd, &mut file_data_tx).await;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;
    use steady_state::*;
    use super::*;

    #[async_std::test]
    pub(crate) async fn test_simple_process() {
        let mut graph = GraphBuilder::for_testing().build(());
        let (file_data_tx, test_file_data_rx) = graph.channel_builder().with_capacity(4).build();
        let state = new_state();
        graph.actor_builder()
            .with_name("UnitTest")
            .build_spawn(move |context| internal_behavior(context, file_data_tx.clone(), state.clone()));
        graph.start();
        graph.request_stop();
        graph.block_until_stopped(Duration::from_secs(15));
        let _results_file_data_vec = test_file_data_rx.testing_take().await;
    }
}
