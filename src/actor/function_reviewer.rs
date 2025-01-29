#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::function_scraper::CodeFunction;
use std::fs::File;
use std::io::{BufRead, BufReader};


use surf::Client;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use dotenv::dotenv;
use std::env;

use surf::http::headers::HeaderValue;
use surf::http::headers::AUTHORIZATION;



#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ReviewedFunction {
    pub name: String,
    pub namespace: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub review_message: String,
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionreviewerInternalState {
}

#[derive(Debug, Deserialize)]
pub struct ReviewResponse {
    pub function_name: String,
    pub review: String,
    pub continue_flag: i32,
    pub next_function: String,
    pub next_function_path: String,
}

async fn get_function_content(filepath: &str, start_line: usize, end_line: usize) -> Result<String, Box<dyn Error>> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines()
        .collect::<Result<Vec<_>, _>>()?;
    
    let content = lines[start_line - 1..end_line]
        .join("\n");
    
    Ok(content)
}

// Function to call the OpenAI API asynchronously and get a response
// pub async fn call_openai_api(prompt: &str, api_key: &str) -> Result<String, Box<dyn Error>> {

//     // Create an HTTP client using the surf crate
//     let client = Client::new();

//     // Send the request to the OpenAI API
//     let mut response = client
//         .post("https://api.openai.com/v1/chat/completions")  // API endpoint for OpenAI
//         .header("Authorization", format!("Bearer {}", api_key)) // Add authorization header with the API key
//         .header("Content-Type", "application/json") // Set the request content type to JSON
//         .body(surf::Body::from_json(&json!({ // Create the JSON request body
//             "model": "gpt-4", // Specify the AI model
//             "messages": [{"role": "user", "content": prompt}], // Include the user's prompt
//             "max_tokens": 100 // Limit the number of response tokens
//         }))?)
//         .await?; // Await the response from the API

//     // Parse the response as JSON
//     let response_json: serde_json::Value = response.body_json().await?;

//     // Line below is used to see raw output from api for troubleshooting
//     // println!("API Response: {:?}", response_json);

//     // Extract the AI's response content from the JSON response
//     let response_content = match response_json["choices"][0]["message"]["content"].as_str() {
//         Some(content) => content.to_string(),
//         None => "No response received.".to_string(),
//     };

//     Ok(response_content)
// }



async fn send_prompt_to_chatgpt(prompt: &str) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").expect("API key not found in environment variables");

    let api_url = "https://api.openai.com/v1/chat/completions";
    let client = Client::new();

    let request_body = json!({
        "model": "gpt-4o-mini", // Specify the model
        "messages": [
            {
                 "role": "system",
                "content": "You are an AI assistant specializing in code analysis."
            },
            {
                 "role": "user",
                "content": prompt
            }
        ],
    "max_tokens": 1000,
        "temperature": 0.0
    });

    let mut response = client
        .post(api_url)
        .header(AUTHORIZATION, format!("Bearer {}", api_key))
        .body(surf::Body::from_json(&request_body)?)
        .await?;

    if response.status().is_success() {
        let response_body: JsonValue = response.body_json().await?;
        if let Some(choice) = response_body.get("choices").and_then(|choices| choices.get(0)) {
            if let Some(content) = choice.get("message").and_then(|msg| msg.get("content")) {
                return Ok(content.as_str().unwrap_or("").to_string());
            }
        }
        Err("Failed to parse ChatGPT response".into())
    } else {
        let error_message = response.body_string().await?;
        Err(format!("API request failed: {}", error_message).into())
    }
}


// pub async fn review_function(
//     api_key: &str, 
//     func: &CodeFunction, 
//     remaining_functions: &[CodeFunction]
// ) -> Result<ReviewedFunction, Box<dyn Error>> {
//     //! Change in the future this Result to ReviewResponse return
//     let function_content = get_function_content(&func.filepath, func.start_line, func.end_line).await?;
    
//     let non_reviewed_list = remaining_functions
//         .iter()
//         .map(|f| format!("{}, {}", f.name, f.filepath))
//         .collect::<Vec<_>>()
//         .join("\n");

//     let prompt = format!(
//         "I am conducting an AI Code Review of my entire project base. Here is the content of the function you thought it was important. \
//         Along with its path.\n\n{}\n{}\n\n\
//         I want you to provide me a review of this piece of code in 200 words or less. The next step for you would be to decide the next function you want to look at. \
//         Here is the list of not reviewed functions:\n\
//         {}\n\n\
//         I want the format of your message to be like this. No other text and explanation.\n\
//         {{function_name, review of the current function, flag if you want to review next function(0 if you are done with the entire review, and 1 if you want to read more functions), next_function, next_function_path}}",
//         function_content,
//         func.filepath,
//         non_reviewed_list
//     );

//     let response = call_openai_api(api_key, &prompt).await?;
//     let return_value = ReviewedFunction {

//         name: func.name.clone(),
//         namespace: String::from("===TEST==="),
//         filepath: func.filepath.clone(),
//         start_line: func.start_line,
//         end_line: func.end_line,
//         review_message: response,

//     };
//     println!("RESPONSE IN review_function() {:?}", return_value);
//     Ok(return_value)
// }
// let response = call_openai_api(api_key, &prompt).await?;
// let return_value = ReviewedFunction {

//     name: func.name.clone(),
//     namespace: String::from("===TEST==="),
//     filepath: func.filepath.clone(),
//     start_line: func.start_line,
//     end_line: func.end_line,
//     review_message: response,

// };
// println!("RESPONSE IN review_function() {:?}", return_value);
// Ok(return_value)
// }

pub async fn run(context: SteadyContext
        ,functions_rx: SteadyRx<CodeFunction>
        ,reviewed_tx: SteadyTx<ReviewedFunction>, state: SteadyState<FunctionreviewerInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [functions_rx],[reviewed_tx]);
  internal_behavior(cmd,functions_rx, reviewed_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,functions_rx: SteadyRx<CodeFunction>,reviewed_tx: SteadyTx<ReviewedFunction>, state: SteadyState<FunctionreviewerInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionreviewerInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut functions_rx = functions_rx.lock().await;
   let mut reviewed_tx = reviewed_tx.lock().await;



   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||functions_rx.is_closed_and_empty() && reviewed_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut functions_rx,1)    );

  
          //TODO:  here is an example reading from functions_rx
          match cmd.try_take(&mut functions_rx) {
              Some(rec) => {

                // let reviewed = ReviewedFunction {
                //     name: rec.name,
                //     namespace: String::from("TEST NAMESPACE"),
                //     filepath: rec.filepath,
                //     start_line: rec.start_line,
                //     end_line: rec.end_line,
                //     review_message: String::from("SOMETHING COOL"),
                // };

                    //? remaing_funciotn is an empyt array
                    let remaining_functions: &[CodeFunction] = &[];
                    let api_key = "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";

                    let reviewed = send_prompt_to_chatgpt(, &rec, remaining_functions).await?;

                  println!("got rec: {:?}", &rec);

                    //TODO:  here is an example writing to reviewed_tx
                    match cmd.try_send(&mut reviewed_tx, reviewed) {
                        Ok(()) => {
                            println!("SENT TO ARCHIVE ACTOR")
                        },
                        Err(msg) => { //in the above await we should have confirmed space is available
                            trace!("error sending: {:?}", msg)
                        },
                    }
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                     println!("ERROR RX WAS NOT RECIEVED")
                  }
              }
          }

        //   let test_function = ReviewedFunction {
        //     name: "example_function".to_string(),
        //     namespace: "example_namespace".to_string(),
        //     filepath: "src/example.rs".to_string(),
        //     start_line: 42,
        //     end_line: 84,
        //     review_message: "This function has been reviewed and approved.".to_string(),
        // };
  
  
        // //TODO:  here is an example writing to reviewed_tx
        // match cmd.try_send(&mut reviewed_tx, test_function) {
        //     Ok(()) => {
        //     },
        //     Err(msg) => { //in the above await we should have confirmed space is available
        //         trace!("error sending: {:?}", msg)
        //     },
        // }
  

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
       let (test_functions_tx,functions_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (reviewed_tx,test_reviewed_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, functions_rx.clone(), reviewed_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_functions_tx.testing_send_all(vec![CodeFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_reviewed_rx.testing_avail_units().await, 1); // check expected count
       let results_reviewed_vec = test_reviewed_rx.testing_take().await;
        }
}