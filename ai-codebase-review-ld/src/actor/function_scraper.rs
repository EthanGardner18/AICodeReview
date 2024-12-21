
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::ArchivedFunction;
use crate::actor::parse_function::ParsedFunction;

use surf::Client;
use serde_json::json;
use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ScrapedFunction {
   //_dummy: u8 //TODO:  remove dummy and put your channel message fields here
   name: String,
   filepath: String,
   start_line: usize,
   end_line: usize,
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionscraperInternalState {
}


pub async fn run(context: SteadyContext
        ,archived_function_rx: SteadyRx<ArchivedFunction>
        ,parsed_function_rx: SteadyRx<ParsedFunction>
        ,scraped_function_tx: SteadyTx<ScrapedFunction>, state: SteadyState<FunctionscraperInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [archived_function_rx,parsed_function_rx],[scraped_function_tx]);
  internal_behavior(cmd,archived_function_rx,parsed_function_rx, scraped_function_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,archived_function_rx: SteadyRx<ArchivedFunction>,parsed_function_rx: SteadyRx<ParsedFunction>,scraped_function_tx: SteadyTx<ScrapedFunction>, state: SteadyState<FunctionscraperInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionscraperInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut archived_function_rx = archived_function_rx.lock().await;
   let mut parsed_function_rx = parsed_function_rx.lock().await;
   let mut scraped_function_tx = scraped_function_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||archived_function_rx.is_closed_and_empty() && parsed_function_rx.is_closed_and_empty() && scraped_function_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut parsed_function_rx,1)    );

  
          //TODO:  here is an example reading from archived_function_rx
          match cmd.try_take(&mut archived_function_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
          //TODO:  here is an example reading from parsed_function_rx
          match cmd.try_take(&mut parsed_function_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
        //TODO:  here is an example writing to scraped_function_tx
        match cmd.try_send(&mut scraped_function_tx, ScrapedFunction::default() ) {
            Ok(()) => {
            },
            Err(msg) => { //in the above await we should have confirmed space is available
                trace!("error sending: {:?}", msg)
            },
        }
  

      }
    }
    Ok(())
}


fn extract_function_details(file_path: &str) -> io::Result<Vec<ScrapedFunction>> {
    let mut function_details = Vec::new();
    let re = Regex::new(r#"\{"([^:]+:[^"]+)",\s*"([^"]+)",\s*(\d+),\s*(\d+)\}"#)
        .expect("Failed to compile regex");

    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = re.captures(&line) {
            let name = caps.get(1).map_or("", |m| m.as_str()).to_string();
            let filepath = caps.get(2).map_or("", |m| m.as_str()).to_string();
            let start_line = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap_or(0));
            let end_line = caps.get(4).map_or(0, |m| m.as_str().parse().unwrap_or(0));

            function_details.push(ScrapedFunction {
                name,
                filepath,
                start_line,
                end_line,
            });
        }
    }

    Ok(function_details)
}


fn get_function_content(filepath: &str, start_line: usize, end_line: usize) -> Result<String, String> {
    let path = Path::new(filepath);

    // Open the file
    let file = File::open(path).map_err(|e| format!("Error opening file {}: {}", filepath, e))?;
    let reader = io::BufReader::new(file);

    // Collect lines into a vector
    let lines: Vec<String> = reader
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if index + 1 >= start_line && index + 1 <= end_line {
                line.ok()
            } else {
                None
            }
        })
        .collect();

    // Join lines into a single string
    Ok(lines.join("\n"))
}


pub async fn get_most_important_function(api_key: &str) -> Result<String, Box<dyn Error>> {

    let function_details = extract_function_details("/Misc/projects/ai-code-view")?;

   let prompt = format!(
        "I am conducting an extensive AI Code Review. Here are the list of functions in my codebase.\n\
        I want you to pick the most important function to start the AI Code review with. I want the message\n\
        to be just the name of the function. No explanation, no other text. Here is the list of functions:\n{}",
        function_details.iter()
            .map(|fd| fd.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    // Create an HTTP client using the surf crate
    let client = Client::new();

    // Send the request to the OpenAI API
    let mut response = client
        .post("https://api.openai.com/v1/chat/completions")  // API endpoint for OpenAI
        .header("Authorization", format!("Bearer {}", api_key)) // Add authorization header with the API key
        .header("Content-Type", "application/json") // Set the request content type to JSON
        .body(surf::Body::from_json(&json!({ // Create the JSON request body
            "model": "gpt-3.5-turbo", // Specify the AI model
            "messages": [{"role": "user", "content": prompt}], // Include the user's prompt
            "max_tokens": 100 // Limit the number of response tokens
        }))?)
        .await?; // Await the response from the API

    // Parse the response as JSON
    let response_json: serde_json::Value = response.body_json().await?;

    // Line below is used to see raw output from api for troubleshooting
    // println!("API Response: {:?}", response_json);

    // Extract the AI's response content from the JSON response
    let response_content = match response_json["choices"][0]["message"]["content"].as_str() {
        Some(content) => content.to_string(),
        None => "No response received.".to_string(),
    };

    Ok(response_content)
}



#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;
    use steady_state::*;
    use super::*;

    #[async_std::test]
    pub(crate) async fn test_simple_process() {
       let mut graph = GraphBuilder::for_testing().build(());
       let (test_archived_function_tx,archived_function_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (test_parsed_function_tx,parsed_function_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (scraped_function_tx,test_scraped_function_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, archived_function_rx.clone(), parsed_function_rx.clone(), scraped_function_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_archived_function_tx.testing_send_all(vec![ArchivedFunction::default()],true).await;

        
       //TODO:  adjust this vec content to make a valid test
       test_parsed_function_tx.testing_send_all(vec![ParsedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_scraped_function_rx.testing_avail_units().await, 1); // check expected count
       let results_scraped_function_vec = test_scraped_function_rx.testing_take().await;
        }
}