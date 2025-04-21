/*
    File: parse_function_actor.rs

    Description:
    This actor is responsible for parsing structured function metadata from source code files
    using OpenAI's GPT models. It listens for incoming `FileData` messages containing 
    numbered code content, sends them to the GPT-4o-mini model with specific parsing instructions, 
    and processes the returned JSON to extract function structure and metadata.

    Responsibilities:
    - Receives line-numbered source code and associated file path through `file_data_rx`
    - Sends a prompt to the OpenAI API to identify function definitions and their line spans
    - Appends raw JSON response to a local file ("parse_function.txt")
    - On receiving the final file (`last_file == "T"`), reprocesses all extracted functions to prepare final structured output
    - Sends a `ParsedCode` message through `parsed_code_tx` for further processing

    Key Features:
    - Two stages of AI interaction:
        1. Extract function headers from line-numbered code
        2. Refine/validate headers once all files are processed
    - Works asynchronously using `steady_state` actors and channels
    - Uses dotenv to load sensitive API keys securely from environment
    - Handles API errors and malformed responses gracefully with debug logging

    Usage:
    - Ensure `.env` file includes `OPENAI_API_KEY`
    - This actor is triggered automatically within a steady-state graph via `run()`
    - Receives input via `FileData`, emits results via `ParsedCode`

    Related Modules:
    - read_file.rs: Sends line-numbered source files to this actor
    - function_scraper.rs (or equivalent): Receives the `ParsedCode` message for downstream use

    Notes:
    - Output JSON is expected to match the format:
      {"function_name", "absoluteFilePath", startLine, endLine}
      or {"className:functionName", ...} for class methods.
    - Final parsing logic is triggered only when the last file chunk is detected.
*/

#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::read_file::FileData;
use serde_json::Value as JsonValue;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::fs;
use async_std::task;


#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ParsedCode {
   pub first_function: String, //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ParsefunctionInternalState {
}


pub async fn run(context: SteadyContext
        ,file_data_rx: SteadyRx<FileData>
        ,parsed_code_tx: SteadyTx<ParsedCode>, state: SteadyState<ParsefunctionInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [file_data_rx],[parsed_code_tx]);
  internal_behavior(cmd,file_data_rx, parsed_code_tx, state).await
}


/*
    Function: chatgpt_first_function

    Description:
    Sends a request to the OpenAI GPT model (gpt-4o-mini) to extract the structural 
    details of functions from a chunk of source code. The input is a formatted string 
    of code (with line numbers and a file path), and the model is prompted to return 
    structured JSON entries for each function found in the code.

    Parameters:
    json: A string (`&str`) containing the file path and numbered source code to be analyzed.

    Returns:
    Result<JsonValue, Box<dyn Error>>: On success, returns the parsed JSON output from the model 
    containing function headers and their line numbers. On failure, returns an error message.

    Notes:
    - Relies on the `OPENAI_API_KEY` from the `.env` file for authorization.
    - Uses a detailed system prompt to instruct the model on formatting and expectations.
    - Ensures only valid JSON structures are returned — no extra explanations or markdown.
    - Uses the `surf` HTTP client to make a POST request to the OpenAI API.

    Example:
    let response = chatgpt_first_function(&formatted_code_string).await?;
    println!("{}", response);
*/

async fn chatgpt_first_function(json: &str) -> Result<JsonValue, Box<dyn Error>> {
    // Loads environment variables from the `.env` file into the process environment.
    // This allows access to secrets like API keys without hardcoding them.
    dotenv::dotenv().ok();  
    // Retrieves the OpenAI API key from the loaded environment variables.
    // If the key is missing, the program will panic with a clear error message.        
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");
    // Sets the endpoint URL for OpenAI's chat completion API.
    // This is the URL the POST request will be sent to.
    let api_url = "https://api.openai.com/v1/chat/completions";
    //The template of the prompt we are using
    let prompt_template = r#"
You are a precise and experienced Code Structure Extraction Agent. You will be given source code in any programming language. The very first line of input will always contain the absolute file path of the code you are analyzing.

Each subsequent line of code will be prefixed with an accurate line number (e.g., `42: <code>`). You can rely on these line numbers for precise tracking — you do not need to count or infer them yourself. These line numbers exist to make parsing easier and help you focus purely on structural detection.

Your task is to parse the code and extract a structured header for every function it contains. For each function, return a JSON-style structure using the format specified below. Your primary goal is accurate start and end line number detection for **only the actual code**, while still counting blank and comment-only lines in the line number range.

CRITICAL RULES:
1. Ignore comments (single-line and multi-line) when identifying the logical start and end of function bodies. However, these lines must still be included in line number range.
2. Blank lines and lines with only comments must still be counted toward line numbers.
3. For class methods, prefix the function name with the class name and a colon (className:functionName).
4. Always use the exact full input file path from the first line. Do not modify, truncate, or abbreviate it.
5. If a function is nested within a class, object, or module, always include the container name, even in languages like Python or JavaScript.
6. Every output entry must strictly follow the formatting structure provided. No extra formatting, explanations, or markdown.

=== FORMATTING SPECIFICATION ===

For class methods:
{"className:functionName", "absoluteFilePath", startLine, endLine}

For top-level (non-class) functions:
{"functionName", "absoluteFilePath", startLine, endLine}

=== EXAMPLE INPUT ===

File path: /src/main.cpp  
Code:
1: int main()  
2: {  
3:   return 0;  
4: }  
5: int sum(int a, int b) {  
6:   return (a + b);  
7: }

=== EXAMPLE OUTPUT ===
{"main", "/src/main.cpp", 1, 4}
{"sum", "/src/main.cpp", 5, 7}

REMEMBER:
- Use the line numbers provided on the left of each line. Never attempt to count manually.
- Your output must contain only JSON structures — no comments, headers, or explanations.
- Be concise, accurate, and consistent.
"#;


    // Create a new HTTP client instance using the `surf` crate
let client = surf::Client::new();

// Construct the JSON request body with the model, messages, and configuration
let request_body = json!({
    "model": "gpt-4o-mini", // Specify the OpenAI model to use
    "messages": [
        // System message sets the behavior and role of the assistant
        { "role": "system", "content": "You are a highly accurate AI-powered code reviewer." },
        
        // User message includes the prompt template and the actual code input
        { "role": "user", "content": format!(
            "{}\n\n{}\n\nJSON List of Functions:\n{}",
            prompt_template.trim(), // Instructions and formatting guide
            "Review this list according to the instructions above.",
            json // Actual code to be analyzed
        )}
    ],
    "max_tokens": 1000, // Limit the length of the response
    "temperature": 0.0   // Set to 0 for deterministic, consistent output
});

// Send the POST request to the OpenAI API endpoint with headers and request body
let mut response = client
    .post(api_url)
    .header("Authorization", format!("Bearer {}", api_key)) // Attach the API key
    .body(surf::Body::from_json(&request_body)?) // Send the JSON as the request body
    .await?; // Await the response asynchronously

// If the API responds with success (200 OK)
if response.status().is_success() {
    // Parse the response body as JSON and return it
    let response_body: JsonValue = response.body_json().await?;
    Ok(response_body)
} else {
    // On failure, capture the error message and return it as a boxed error
    let error_message = response.body_string().await?;
    Err(format!("API request failed: {}", error_message).into())
}

}

/*
    File: chatgpt_function_parser.rs

    Description:
    This module defines an asynchronous function that interacts with the OpenAI ChatGPT API 
    to analyze and extract structured metadata about functions from a given source code file. 
    The input includes the file's content and absolute path, and the output is a JSON array 
    indicating function names, start and end line numbers, and the file path — with support 
    for both top-level functions and class methods.

    Key Features:
    - Sends a detailed prompt with guidelines for parsing and formatting
    - Automatically includes environment-based OpenAI API key via dotenv
    - Formats the response to match a strict, newline-separated JSON structure
    - Supports handling functions nested inside classes
    - Counts comment and blank lines toward line numbers without treating them as code

    Usage:
    Set `OPENAI_API_KEY` in your `.env` file. Call `call_chatgpt_api(file_content, file_path)` 
    with the content of a code file and its full path. The response will contain JSON-formatted 
    function metadata for downstream processing.
*/


async fn call_chatgpt_api(file_content: &str, file_path: &str) -> Result<JsonValue, Box<dyn Error>> {
    // Loads environment variables from the `.env` file into the process environment.
    // This allows access to secrets like API keys without hardcoding them.
    dotenv::dotenv().ok();  
    // Retrieves the OpenAI API key from the loaded environment variables.
    // If the key is missing, the program will panic with a clear error message.        
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");
    // Sets the endpoint URL for OpenAI's chat completion API.
    // This is the URL the POST request will be sent to.
    let api_url = "https://api.openai.com/v1/chat/completions";
    //PROMPT TEMPLATE
    let prompt_template = r#"
        You will receive a file of any coding language, the first line will have the path to the file you are looking at. I would like you to parse the code and only store a header for each function in this format. One
        issue you need to check for is that there are comments in the code, so you need to make sure you are starting at the correct line number and ending at the correct line number. Don't forget that different coding
        languages use different methods to comment things in and out. Also if you see a new line assume it counts toward the total line number count. Finally, if the function is within a class, give the class_name:function name. 

        For a function within a class:
        {class_name:function_name, path, starting_line_number, last_line_number}

        For a function without a class:
        {function_name, path, starting_line_number, last_line_number}

        In your response, you should not have anything besides the JSON format structure.
        For example, if the code for the given filepath /src/main.cpp is
        1 int main()
        2 {
        3   return 0;
        4 }
        5 int sum(int a, int b) {
        6  return (a + b);
        7 }
        Then the, JSON structure should look like this
        {"main", "/src/main.cpp",1,4}
        {"sum", "src/main.cpp",5,7}
    "#;

    // Create an HTTP client using the `surf` crate for making API requests
let client = surf::Client::new();

// Construct the JSON body to be sent to the OpenAI Chat API
// - The system prompt defines the assistant's role as a function-parsing expert
// - The user prompt includes instructions, the file path, and the file content
// - The temperature is set to 0.0 for deterministic (non-random) output
let request_body = json!({
    "model": "gpt-4o-mini",
    "messages": [
        { "role": "system", "content": "You are a code parser specializing in analyzing functions." },
        { "role": "user", "content": format!(
            "{}\n\n{}\n\nFile Path: {}\n\nFile Content:\n{}",
            prompt_template.trim(),
            "Parse this file according to the format above.",
            file_path,
            file_content
        )}
    ],
    "max_tokens": 450,     // Limits the response size to 450 tokens
    "temperature": 0.0     // Ensures consistent and predictable outputs
});

// Send a POST request to the OpenAI API with the appropriate headers and body
let mut response = client
    .post(api_url)
    .header("Authorization", format!("Bearer {}", api_key))   // Attach API key for authentication
    .body(surf::Body::from_json(&request_body)?)              // Convert request body to JSON
    .await?;                                                  // Await the response

// Check if the response status indicates success
if response.status().is_success() {
    // Parse and return the JSON body from the API response
    let response_body: JsonValue = response.body_json().await?;
    Ok(response_body)
} else {
    // If the request failed, return a formatted error with the server's message
    let error_message = response.body_string().await?;
    Err(format!("API request failed: {}", error_message).into())
}

}


/*
    Function: append_to_file

    Description:
    Appends each non-empty line of the provided content to a specified file. 
    Leading/trailing whitespace is trimmed from each line, and optional trailing 
    commas are removed before writing. If the file does not exist, it will be created.

    Parameters:
    file_path: A string slice (`&str`) representing the path to the file to append to.
    content: A string slice (`&str`) representing the multiline content to append.

    Returns:
    std::io::Result<()>: Returns `Ok(())` on success, or an I/O error if the file 
    cannot be opened or written to.

    Notes:
    - Lines that are empty after trimming are skipped.
    - Each valid line is written on a new line in the file.
    - Uses `OpenOptions` to enable file creation and appending.

    Example:
    append_to_file("output.txt", "hello,\nworld,\n") 
    // Appends:
    // hello
    // world
*/

fn append_to_file(file_path: &str, content: &str) -> std::io::Result<()> {
    // Open the file in append mode; create it if it doesn't exist.
    // This allows new content to be added to the end of the file.
    let mut file = OpenOptions::new()
        .create(true)   // Create the file if it doesn't exist
        .append(true)   // Open the file in append mode (adds to the end)
        .open(file_path)?; // Return an error if the file can't be opened

    // Iterate through each line in the provided content string
    for line in content.lines() {
        // Clean the line by trimming leading/trailing spaces
        // and removing a trailing comma if present
        let cleaned_line = line.trim()
            .trim_end_matches(','); // Optional: strip trailing commas

        // Only write non-empty lines to the file
        if !cleaned_line.is_empty() {
            writeln!(file, "{}", cleaned_line)?; // Write the cleaned line followed by a newline
        }
    }

    // Return success if all lines are processed without error
    Ok(())
}


async fn internal_behavior<C: SteadyCommander>(mut cmd: C,file_data_rx: SteadyRx<FileData>,parsed_code_tx: SteadyTx<ParsedCode>, state: SteadyState<ParsefunctionInternalState>
 ) -> Result<(),Box<dyn Error>> {
    let mut state_guard = steady_state(&state, || ParsefunctionInternalState::default()).await;
    if let Some(_state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut file_data_rx = file_data_rx.lock().await;
   let mut parsed_code_tx = parsed_code_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||file_data_rx.is_closed_and_empty() && parsed_code_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut file_data_rx,1)    );

  
          //TODO:  here is an example reading from file_data_rx
          match cmd.try_take(&mut file_data_rx) {
              Some(rec) => {
                trace!("got rec: {:?}", rec);
                  eprintln!("Message received successfully in the parse function actor");
                  

                let response = call_chatgpt_api(&rec.content, &rec.path).await;
    

              match response {
                    Ok(json) => {
                        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                            let content_cleaned = content
                                .strip_prefix("```json\n")
                                .and_then(|s| s.strip_suffix("```"))
                                .unwrap_or(content)
                                .trim();

                            // Extract only what's inside the brackets []
                            let content_inside_brackets = content_cleaned
                                .strip_prefix('[')
                                .and_then(|s| s.strip_suffix(']'))
                                .unwrap_or(content_cleaned)
                                .trim();

                        

                            if let Err(e) = append_to_file("parse_function.txt", content_inside_brackets) {
                                eprintln!("Failed to write to file: {}", e);
                            }
                        } else {
                            eprintln!("Unexpected JSON structure: {}", json);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to call ChatGPT API: {}", e);
                    }
                }

                let mut _test = String::new();
                let _final_message1 = "sample stuff".to_string();
                if rec.last_file == "T" {
                _test = fs::read_to_string("parse_function.txt")
                    .expect("Failed to read file");
                eprintln!("File content when the stuff is True checkMark:\n{}", _test);

                // Call the async function and print the response
              let response = task::block_on(async {
                    chatgpt_first_function(&_test).await.ok()
                }).unwrap_or_else(|| json!({"choices": [{"message": {"content": ""}}]}));
                
                // Print the response
                let final_message = response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                let data = ParsedCode {
                 first_function: final_message,
                };
                 match cmd.try_send(&mut parsed_code_tx, data.clone() ) {
                        Ok(()) => {
                            eprintln!("First function to be reviewed sent to Function_scraper actor: {:?}", data);
                        },
                        Err(msg) => { //in the above await we should have confirmed space is available
                            trace!("error sending: {:?}", msg)
                        },
                    }
                parsed_code_tx.mark_closed();
                eprintln!("Parsed code channel closed.");

                // **Break out of the loop**
                break;
                



            } else {
                eprintln!("Not the last file, skipping read.");
            }



              }
              None => {
                  if clean {
                     eprintln!("ERROR RX WAS NOT RECIEVED");
                  }
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
       let (test_file_data_tx,file_data_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (parsed_code_tx,test_parsed_code_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, file_data_rx.clone(), parsed_code_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_file_data_tx.testing_send_all(vec![FileData::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_parsed_code_rx.testing_avail_units().await, 1); // check expected count
       let _results_parsed_code_vec = test_parsed_code_rx.testing_take().await;
        }
}