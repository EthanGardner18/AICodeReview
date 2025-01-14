#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// If no internal state is required (recommended), feel free to remove this.
#[derive(Default)]
pub(crate) struct InputprinterInternalState {}

#[cfg(not(test))]
pub async fn run(
    context: SteadyContext,
    state: SteadyState<InputprinterInternalState>,
) -> Result<(), Box<dyn Error>> {
    // If needed, CLI Args can be pulled into state from _cli_args
    let _cli_args = context.args::<Args>();
    // Monitor consumes context and ensures all the traffic on the chosen channels is monitored
    // Monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
    let cmd = into_monitor!(context, [], []);
    internal_behavior(cmd, state).await
}

// Recursive function to print all files with specific extensions in a directory and its subdirectories
fn print_filtered_files_in_directory(path: &Path, extensions: &[&str]) {
    if path.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            // Recursive call for subdirectory
                            print_filtered_files_in_directory(&entry_path, extensions);
                        } else if let Some(ext) = entry_path.extension() {
                            if let Some(ext_str) = ext.to_str() {
                                if extensions.contains(&ext_str) {
                                    println!("{}", entry_path.display());
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to read directory '{}': {}", path.display(), e);
            }
        }
    }
}

// Function to read and print the contents of a file
fn read_and_print_file(file_path: &Path) {
    if file_path.is_file() {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                println!("Contents of file '{}':\n{}", file_path.display(), content);
            }
            Err(e) => {
                println!("Failed to read file '{}': {}", file_path.display(), e);
            }
        }
    } else {
        println!("The provided path '{}' is not a valid file.", file_path.display());
    }
}

async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    state: SteadyState<InputprinterInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || InputprinterInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {
        // Every read and write channel must be locked for this instance use, this is outside before the loop

        // This is the main loop of the actor, will run until shutdown is requested.
        // The closure is called upon shutdown to determine if we need to postpone the shutdown
        while cmd.is_running(&mut || true) {
            // Ask the user for a directory path
            print!("Enter a directory path (or type 'exit' to quit): ");
            io::stdout().flush()?; // Ensure the prompt is displayed immediately
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            // Exit condition
            if input.eq_ignore_ascii_case("exit") {
                println!("Exiting program.");
                break;
            }

            // Define extensions for coding languages
            let coding_extensions = [
                "py", "cpp", "h", "hpp", "cc", "cxx", "rs", "c", "js", "jsx", "ts", "tsx", "java",
                "go", "html", "htm", "css", "sh", "php", "rb", "kt", "kts", "swift", "pl", "pm",
                "r", "md",
            ];

            // Check if the path is valid and list all relevant files
            let path = Path::new(input);
            if path.is_dir() {
                println!("Contents of directory '{}' (filtered by coding extensions):", input);
                print_filtered_files_in_directory(path, &coding_extensions);

                // Prompt user to select a file for reading
                print!("Enter the path of a file to read its content (or type 'skip' to skip): ");
                io::stdout().flush()?;
                let mut file_input = String::new();
                io::stdin().read_line(&mut file_input)?;
                let file_input = file_input.trim();

                if file_input.eq_ignore_ascii_case("skip") {
                    continue;
                }

                let file_path = Path::new(file_input);
                read_and_print_file(file_path);
            } else {
                println!("The provided path is not a valid directory.");
            }

            // Avoid spinning the loop by using an await for periodic checks
            let _clean = await_for_all!(cmd.wait_periodic(Duration::from_millis(1000)));
        }
    }
    Ok(())
}

#[cfg(test)]
pub async fn run(
    context: SteadyContext,
    state: SteadyState<InputprinterInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut cmd = into_monitor!(context, [], []);
    if let Some(responder) = cmd.sidechannel_responder() {
        while cmd.is_running(&mut || true) {}
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
        let state = new_state();
        graph
            .actor_builder()
            .with_name("UnitTest")
            .build_spawn(move |context| internal_behavior(context, state.clone()));

        graph.start(); // Start up the graph
        graph.request_stop();
        graph.block_until_stopped(Duration::from_secs(15));
    }
}
