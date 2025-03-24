
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::read_file::FileData;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::fs;
use async_std::task;


#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ParsedCode {
   pub firstFunction: String, //TODO:  remove dummy and put your channel message fields here
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

async fn chatgpt_firstfunction(json: &str) -> Result<JsonValue, Box<dyn Error>> {
    dotenv::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");

    let api_url = "https://api.openai.com/v1/chat/completions";

    let prompt_template = r#"
You are a precise and experienced Code Structure Extraction Agent. You will be given source code in any programming language. The very first line of input will contain the absolute file path for the code you are analyzing.

Your task is to parse this file and extract a structured header for each function it contains. You must identify and output a JSON-style structure for every function using the format below. Pay close attention to syntax, nesting, and comment handling. Your primary objective is accurate start and end line number detection for each function.

CRITICAL RULES:
1. Comments (single-line and multi-line) must be ignored when identifying function bodies. Start and end lines must reflect only actual code lines, but you must still count commented lines and empty lines toward line number totals.
2. Blank lines and lines with only comments must be counted toward line numbers.
3. For class methods, prefix the function name with the class name and a colon (className:functionName).
4. Maintain the exact input file path from line 1 of the file for every output entry.
5. If a function is nested inside a class or object, include the class/object name, even if it's in a language like Python or JavaScript.
6. Your output must contain only JSON-style entries per function. Do not add explanations, extra formatting, line breaks, or markdown.

=== FORMATTING SPECIFICATION ===

For functions inside a class:
{className:functionName, path, startLine, endLine}

For top-level (non-class) functions:
{functionName, path, startLine, endLine}

=== EXAMPLE INPUT ===

File path: /src/main.cpp  
Code:
1 int main()  
2 {  
3   return 0;  
4 }  
5 int sum(int a, int b) {  
6   return (a + b);  
7 }

=== EXAMPLE OUTPUT ===
{"main", "/src/main.cpp", 1, 4}
{"sum", "/src/main.cpp", 5, 7}

YOUR RESPONSE MUST ONLY CONTAIN THESE JSON STRUCTURES FOR EACH FUNCTION FOUND IN THE CODE. DO NOT ADD ANY HEADERS, DESCRIPTIONS, OR EXTRA TEXT.
"#;


    let client = surf::Client::new();
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            { "role": "system", "content": "You are a highly accurate AI-powered code reviewer." },
            { "role": "user", "content": format!(
                "{}\n\n{}\n\nJSON List of Functions:\n{}",
                prompt_template.trim(),
                "Review this list according to the instructions above.",
                json
            )}
        ],
        "max_tokens": 1000,
        "temperature": 0.0
    });

    let mut response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .body(surf::Body::from_json(&request_body)?)
        .await?;

    if response.status().is_success() {
        let response_body: JsonValue = response.body_json().await?;
        Ok(response_body)
    } else {
        let error_message = response.body_string().await?;
        Err(format!("API request failed: {}", error_message).into())
    }
}

async fn call_chatgpt_api(file_content: &str, file_path: &str) -> Result<JsonValue, Box<dyn Error>> {
    dotenv::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");

    let api_url = "https://api.openai.com/v1/chat/completions";

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

    let client = surf::Client::new();
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
        "max_tokens": 450,
        "temperature": 0.0
    });

    let mut response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .body(surf::Body::from_json(&request_body)?)
        .await?;

    if response.status().is_success() {
        let response_body: JsonValue = response.body_json().await?;
        Ok(response_body)
    } else {
        let error_message = response.body_string().await?;
        Err(format!("API request failed: {}", error_message).into())
    }
}


fn append_to_file(file_path: &str, content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    for line in content.lines() {
        let cleaned_line = line.trim() // Removes leading and trailing spaces
            .trim_end_matches(','); // Optional: Removes trailing comma if you don't want it

        if !cleaned_line.is_empty() {
            writeln!(file, "{}", cleaned_line)?;
        }
    }

    Ok(())
}


async fn internal_behavior<C: SteadyCommander>(mut cmd: C,file_data_rx: SteadyRx<FileData>,parsed_code_tx: SteadyTx<ParsedCode>, state: SteadyState<ParsefunctionInternalState>
 ) -> Result<(),Box<dyn Error>> {

    println!("Parse function actor is fired up. ");

    let mut state_guard = steady_state(&state, || ParsefunctionInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

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
                  println!("Message received successfully in the parse function actor");
                  

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

                        

                            if let Err(e) = append_to_file("test.txt", content_inside_brackets) {
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

                let mut test = String::new();
                let final_message1 = "sample stuff".to_string();
                if rec.lastFile == "T" {
                test = fs::read_to_string("test.txt")
                    .expect("Failed to read file");
                println!("File content when the stuff is True checkMark:\n{}", test);

                // Call the async function and print the response
              let response = task::block_on(async {
                    chatgpt_firstfunction(&test).await.ok()
                }).unwrap_or_else(|| json!({"choices": [{"message": {"content": ""}}]}));
                
                // Print the response
                let final_message = response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();
                let data = ParsedCode {
                 firstFunction: final_message,
                };
                 match cmd.try_send(&mut parsed_code_tx, data.clone() ) {
                        Ok(()) => {
                            println!("First function to be reviewed sent to Function_scraper actor: {:?}", data);
                        },
                        Err(msg) => { //in the above await we should have confirmed space is available
                            trace!("error sending: {:?}", msg)
                        },
                    }
                parsed_code_tx.mark_closed();
                println!("Parsed code channel closed.");

                // **Break out of the loop**
                break;
                



            } else {
                println!("Not the last file, skipping read.");
            }



              }
              None => {
                  if clean {
                     println!("ERROR RX WAS NOT RECIEVED");
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
       let results_parsed_code_vec = test_parsed_code_rx.testing_take().await;
        }
}
