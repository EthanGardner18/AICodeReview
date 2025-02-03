#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::read_directory::FileMetadata;
use std::fs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};


#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct FileContent {
pub directory_files:
	HashMap<PathBuf, String>, //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ReadfileInternalState {
}


pub async fn run(context: SteadyContext
                 ,file_list_rx: SteadyRx<FileMetadata>
                 ,file_content_tx: SteadyTx<FileContent>, state: SteadyState<ReadfileInternalState>
) -> Result<(),Box<dyn Error>> {

	// if needed CLI Args can be pulled into state from _cli_args
	let _cli_args = context.args::<Args>();
	// monitor consumes context and ensures all the traffic on the chosen channels is monitored
	// monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
	let cmd =  into_monitor!(context, [file_list_rx],[file_content_tx]);
	internal_behavior(cmd,file_list_rx, file_content_tx, state).await
}



async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    file_list_rx: SteadyRx<FileMetadata>,
    file_content_tx: SteadyTx<FileContent>,
    state: SteadyState<ReadfileInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || ReadfileInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {
        // Lock channels before the loop
        let mut file_list_rx = file_list_rx.lock().await;
        let mut file_content_tx = file_content_tx.lock().await;

        // Main loop of the actor
        while cmd.is_running(&mut || file_list_rx.is_closed_and_empty() && file_content_tx.mark_closed()) {
            let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut file_list_rx, 1));

            // Declare directory_files outside the match block
            let mut directory_files: HashMap<PathBuf, String> = HashMap::new();

            // Read from file_list_rx
            match cmd.try_take(&mut file_list_rx) {
                Some(rec) => {
                    trace!("got rec: {:?}", rec);

                    // Populate directory_files with file contents
                    for path in &rec.found_files {
                        if let Ok(contents) = fs::read_to_string(path) {
                            let numbered_content = contents
                                .lines()
                                .enumerate()
                                .map(|(i, line)| format!("{}: {}", i + 1, line))
                                .collect::<Vec<String>>()
                                .join("\n");

                            directory_files.insert(path.clone(), numbered_content);
                        } else {
                            eprintln!("Failed to read file: {}", path.display());
                        }
                    }

                    // Debug output
                    for (file_path, content) in &directory_files {
                        println!("File: {}\n{}", file_path.display(), content);
                    }
                }
                None => {
                    if clean {
                        // Handle unexpected empty result
                    }
                }
            }

            // Ensure directory_files is always used correctly
            let file_content = FileContent { directory_files };

            match cmd.try_send(&mut file_content_tx, file_content) {
                Ok(()) => {}
                Err(msg) => {
                    trace!("error sending: {:?}", msg);
                }
            }
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
		let (test_file_list_tx,file_list_rx) = graph.channel_builder().with_capacity(4).build();

		let (file_content_tx,test_file_content_rx) = graph.channel_builder().with_capacity(4).build();
		let state = new_state();
		graph.actor_builder()
		.with_name("UnitTest")
		.build_spawn( move |context|
		              internal_behavior(context, file_list_rx.clone(), file_content_tx.clone(), state.clone())
		            );

		graph.start(); //startup the graph
		//TODO:  adjust this vec content to make a valid test
		test_file_list_tx.testing_send_all(vec![FileMetadata::default()],true).await;


		graph.request_stop();
		graph.block_until_stopped(Duration::from_secs(15));
		//TODO:  confirm values on the output channels
		//    assert_eq!(test_file_content_rx.testing_avail_units().await, 1); // check expected count
		let results_file_content_vec = test_file_content_rx.testing_take().await;
	}
}
