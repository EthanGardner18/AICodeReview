

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

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct FileData {
    pub path: String,
    pub content: String,
    pub lastFile: String,
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ReadfileInternalState {
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,file_data_tx: SteadyTx<FileData>, state: SteadyState<ReadfileInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [],[file_data_tx]);
  internal_behavior(cmd, file_data_tx, state).await
}

fn scan_directory_for_files(path: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found_files = Vec::new();
    if path.is_dir() {
        // Read the directory entries
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    // Recursively scan the subdirectory
                    found_files.extend(scan_directory_for_files(&entry_path, extensions));
                } else if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    // Check if the file extension matches one of the desired ones
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

pub fn read_file_with_line_numbers(path: &Path) -> Option<String> {
    if let Ok(contents) = fs::read_to_string(path) {
        let numbered_content = contents
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{}: {}", i + 1, line))
            .collect::<Vec<String>>()
            .join("\n");
        Some(numbered_content)
    } else {
        eprintln!("Failed to read file: {}", path.display());
        None
    }
}

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    file_data_tx: SteadyTx<FileData>,
    state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || ReadfileInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {
        // Lock the write channel outside the loop
        let mut file_data_tx = file_data_tx.lock().await;

        // ✅ Ask for user input **before** the loop
        let mut input_path = String::new();
        println!("Enter the file path:");
        io::stdin().read_line(&mut input_path).expect("Failed to read line");

        // Trim to remove any trailing newline
        let input_path = input_path.trim().to_string();
        let input_dir = PathBuf::from(&input_path);

        // Define allowed file extensions
        let coding_extensions = [
            "py", "cpp", "h", "hpp", "cc", "cxx", "rs", "c", "js", "jsx", "ts", "tsx", "java",
            "go", "html", "htm", "css", "sh", "php", "rb", "kt", "kts", "swift", "pl", "pm",
            "r", "md",
        ];

        // Scan directory for files once
        let found_files = scan_directory_for_files(&input_dir, &coding_extensions);
        let total_files = found_files.len();
        let mut processed_files = 0;

        // ✅ Main loop (runs indefinitely or until shutdown)
        while cmd.is_running(&mut || file_data_tx.mark_closed()) {
            // Prevent spinning by awaiting a periodic check
            let clean = await_for_all!(cmd.wait_periodic(Duration::from_millis(1000)));

            // Process each file in the directory
            for file_path in &found_files {
                processed_files += 1;
                let is_last_file = processed_files == total_files;

                let file_content = read_file_with_line_numbers(file_path).unwrap_or_else(|| {
                    eprintln!("Failed to read file: {}", file_path.display());
                    String::new()
                });

                let data = FileData {
                    path: file_path.display().to_string(),
                    content: file_content,
                    lastFile: if is_last_file { "T".to_string() } else { "F".to_string() },
                };

                match cmd.try_send(&mut file_data_tx, data) {
                    Ok(()) => {
                        println!(
                            "Message sent successfully {}",
                            if is_last_file { "(last file)" } else { "" }
                        );

                        // ✅ If it's the last file, mark the channel as closed
                        if is_last_file {
                            file_data_tx.mark_closed();
                            println!("file_data_tx marked as closed.");
                            println!("All files processed. Exiting actor.");
                            return Ok(()); // Gracefully exit the function
                        }
                    }
                    Err(msg) => trace!("Error sending: {:?}", msg),
                }
            }
        }
    }
    Ok(())
}



#[cfg(test)]
pub async fn run(context: SteadyContext
        ,file_data_tx: SteadyTx<FileData>, state: SteadyState<ReadfileInternalState>
    ) -> Result<(),Box<dyn Error>> {
    let mut cmd =  into_monitor!(context, [],[file_data_tx]);
    if let Some(responder) = cmd.sidechannel_responder() {
         let mut file_data_tx = file_data_tx.lock().await;
         while cmd.is_running(&mut ||file_data_tx.mark_closed()) {
                 // in main use graph.sidechannel_director node_call(msg,"ReadFile")
                 let _did_echo = responder.echo_responder(&mut cmd,&mut file_data_tx).await;
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
       let (file_data_tx,test_file_data_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, file_data_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_file_data_rx.testing_avail_units().await, 1); // check expected count
       let results_file_data_vec = test_file_data_rx.testing_take().await;
        }
}