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
use serde_json::json;
use crate::kv::Value as KvValue; // Alias for crate::kv::Value
use log::kv::Value as LogValue;  // Alias for log::kv::Value
use serde_json::Value as JsonValue; // Alias for serde_json::Value
use surf::Client;
use surf::http::headers::HeaderValue;
use surf::http::headers::AUTHORIZATION;
use dotenv::dotenv;
use std::env;


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


async fn internal_behavior<C: SteadyCommander>(
    mut cmd: C,
    state: SteadyState<InputprinterInternalState>,
) -> Result<(), Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || InputprinterInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {
        let mut directories: Vec<PathBuf> = Vec::new(); // Array to store directories
        let mut results = Vec::new(); // Store the JSON results

        while cmd.is_running(&mut || true) {
            print!("Enter a directory path (or type 'exit' to quit): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("exit") {
                println!("Exiting program.");
                break;
            }

            let coding_extensions = [
                "py", "cpp", "h", "hpp", "cc", "cxx", "rs", "c", "js", "jsx", "ts", "tsx", "java",
                "go", "html", "htm", "css", "sh", "php", "rb", "kt", "kts", "swift", "pl", "pm",
                "r", "md",
            ];

            let path = Path::new(input);
            if path.is_dir() {
                directories.push(path.to_path_buf()); // Add directory to the array

                println!("Contents of directory '{}' (filtered by coding extensions):", input);
                print_filtered_files_in_directory(path, &coding_extensions);

                for dir in &directories {
                    if let Err(e) = process_directory(dir, &mut results).await {
                        eprintln!("Error processing directory {}: {}", dir.display(), e);
                    }
                }

                // Process the JSON results to extract and format content
                let formatted_results: Vec<String> = results
                    .iter()
                    .filter_map(|result| {
                        if let Some(content) = result
                            .get("choices")
                            .and_then(|choices| choices.get(0))
                            .and_then(|choice| choice.get("message"))
                            .and_then(|message| message.get("content"))
                            .and_then(|content| content.as_str())
                        {
                            // Use intermediate bindings to extend lifetimes
                            let content_replaced = content.replace("```", "");
                            let lines = content_replaced.trim();

                            Some(
                                lines
                                    .lines()
                                    .filter(|line| !line.trim().is_empty()) // Skip empty lines
                                    .map(|line| {
                                        // Use intermediate bindings for cleaned lines
                                        let binding = line.replace("{", "").replace("}", "");
                                        let cleaned_line = binding.trim();
                                        let parts: Vec<&str> = cleaned_line.split(',').collect();

                                        if parts.len() == 4 {
                                            format!(
                                                r#"{{"{}","{}","{}","{}"}}"#,
                                                parts[0].trim(),
                                                parts[1].trim(),
                                                parts[2].trim(),
                                                parts[3].trim()
                                            )
                                        } else {
                                            String::new() // Handle malformed lines gracefully
                                        }
                                    })
                                    .filter(|line| !line.is_empty()) // Skip malformed lines
                                    .collect::<Vec<String>>()
                                    .join("\n"),
                            )
                        } else {
                            None
                        }
                    })
                    .collect();

                // Write the formatted results to a file
                fs::write("test.txt", formatted_results.join("\n"))?;

                println!("Formatted results written to test.txt.");
            } else {
                println!("The provided path is not a valid directory.");
            }

            let _clean = await_for_all!(cmd.wait_periodic(Duration::from_millis(1000)));
        }
    }
    Ok(())
}

async fn process_directory(
    dir: &Path,
    results: &mut Vec<serde_json::Value>,
) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_content = read_and_print_file(&path)?;
            let file_path = path.display().to_string(); // Get the file path as a string
            let parsed_output = call_chatgpt_api(&file_content, &file_path).await?;
            results.push(json!(parsed_output));
        }
    }
    Ok(())
}


fn read_and_print_file(path: &Path) -> Result<String, Box<dyn Error>> {
    if let Ok(content) = fs::read_to_string(path) {
        println!("Reading file: {}", path.display());
        println!("{}", content);
        return Ok(content);
    }
    Err("Failed to read file".into())
}


async fn call_chatgpt_api(file_content: &str, file_path: &str) -> Result<JsonValue, Box<dyn Error>> {
    // Your OpenAI API key (read from environment variables for security)
     dotenv().ok();	
     let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");
    
    // The API endpoint for ChatGPT
    let api_url = "https://api.openai.com/v1/chat/completions";
    
    // The prompt template
    let prompt_template = format!(
        "
        You will receive a file of any coding language, the first line will have the path to the file you are looking at. I would like you to parse the code and only store a header for each function in this format. One \
        issue you need to check for is that there are comments in the code, so you need to make sure you are starting at the correct line number and ending at the correct line number. Don't forget that different coding \
        languages use different methods to comment things in and out. Also if you see a new line assume it counts toward the total line number count. Finally, if the function is within a class, give the class \
        name:function name.\n\nFor a function within a class:\n{{class_name:function_name, path, starting_line_number, last_line_number}}\n\nFor a function without a class:\n{{function_name, path, starting_line_number, last_line_number}}\n\nOnly send the output with nothing else.\n\nHere is the content of the file:\n{file_content}\n\nAnd this is the path to the file: {file_path}\n
        "
    );
    
    // HTTP client
    let client = Client::new();
    
    // Construct the JSON payload
    let request_body = json!({
        "model": "gpt-4o-mini", // Specify the model you want to use
        "messages": [
            {
                "role": "system",
                "content": "You are a code parser specializing in analyzing functions."
            },
            {
                "role": "user",
                "content": prompt_template
            }
        ],
        "max_tokens": 1000, // Adjust based on your expected output size
        "temperature": 0.0  // Lower temperature ensures deterministic results
    });
    
    // Make the POST request
    let mut response = client
	.post(api_url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .body(surf::Body::from_json(&request_body)?)
        .await?;    
    // Parse the JSON response
    if response.status().is_success() {
	let response_body: JsonValue = response.body_json().await?;
        Ok(response_body)
    } else {
	let error_message = response.body_string().await?;
        Err(format!("API request failed: {}", error_message).into())
    }
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
