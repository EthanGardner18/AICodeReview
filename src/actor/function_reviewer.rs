#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::function_scraper::CodeFunction;

use surf::Client;
use serde_json::Value as JsonValue;
use dotenv::dotenv;
use std::env;
use std::collections::HashMap;
use surf::http::headers::AUTHORIZATION;
use serde_json::json;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ReviewedFunction {
    pub name: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub review_message: String,
    pub function_map: HashMap<String, String>
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionreviewerInternalState {
}

/*
    Function: send_prompt_to_chatgpt

    Description:
    Sends a prompt to OpenAI's ChatGPT API and retrieves the generated response.
    The function constructs a request, sends it asynchronously, and parses the response.

    Parameters:
    - prompt: &str — A reference to the string containing the prompt to be sent.

    Returns:
    - Result<String, Box<dyn Error>> — Returns Ok(String) containing the ChatGPT response on success.
      On failure, returns an error if the API request fails or the response cannot be parsed.

    Errors:
    - Returns an error if:
      - The API key is missing from environment variables.
      - The API request fails due to network issues or invalid parameters.
      - The response from the API cannot be parsed or lacks expected structure.

    Side Effects:
    - Reads the OpenAI API key from environment variables.
    - Sends a request to "https://api.openai.com/v1/chat/completions".
    - Uses the Surf HTTP client to perform asynchronous network operations.
    - Logs an error message if the API call fails.

    Notes:
    - Uses the GPT-4o-mini model with a maximum token limit of 500.
    - Maintains a low temperature setting (0.0) to encourage deterministic responses.
    - Parses the API response to extract the content of the first returned choice.
*/
async fn send_prompt_to_chatgpt(prompt: &str) -> Result<String, Box<dyn Error>> {
    dotenv().ok();
    let api_key = env::var("OPENAI_API_KEY").expect("API key not found in environment variables");

    let api_url = "https://api.openai.com/v1/chat/completions";
    let client = Client::new();

    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                 "role": "system",
                "content": "Give me an explanation"
            },
            {
                 "role": "user",
                "content": prompt
            }
        ],
        "max_tokens": 500,
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


/*
    Function: review_function

    Description:
    Conducts an AI-driven code review for a given function using OpenAI's ChatGPT.
    It provides structured feedback based on predefined review criteria and severity levels.

    Parameters:
    - func: &CodeFunction — A reference to the function to be reviewed.
    - remaining_functions: &HashMap<String, String> — A reference to a map containing remaining functions to review.

    Returns:
    - Result<ReviewedFunction, Box<dyn Error>> — Returns Ok(ReviewedFunction) with structured AI feedback on success.
      On failure, returns an error if the API request fails or the response cannot be parsed.

    Errors:
    - Returns an error if:
      - The API call fails or responds with an unexpected format.
      - The function data cannot be properly processed.

    Side Effects:
    - Sends a request to OpenAI's ChatGPT API for automated code review.
    - Logs trace messages for debugging purposes.
    - Uses environment variables to retrieve the OpenAI API key.

    Notes:
    - Provides structured code reviews, including severity levels and recommendations.
    - Ensures responses follow a strict format for easy parsing and further automation.
    - Helps developers maintain clarity and improve maintainability in code review workflows.
*/
pub async fn review_function(
    func: &CodeFunction, 
    remaining_functions: &std::collections::HashMap<String, String>
) -> Result<ReviewedFunction, Box<dyn Error>> {
    let function_content = &func.content;
    
    let remaining_functions_list = remaining_functions
        .keys()
        .map(|key| key.to_string())
        .collect::<Vec<String>>()
        .join("\n");

   let prompt = format!(
  "You are an advanced AI Code Review Agent with over 10 years of software engineering experience, fluent in all major programming languages and paradigms. Your role is to conduct an in-depth code review of an entire project, function-by-function, with the expertise and diligence of a seasoned senior engineer. You’re expected to spot bugs, inefficiencies, anti-patterns, and logic flaws while also reviewing the function’s clarity, maintainability, and purpose alignment. You are context-aware and meticulous, paying attention to the function body as well as inline comments to deduce developer intent and possible deviations.

The code review is being conducted in an iterative loop-based structure: At each step, you are given the content of a single function that you previously identified as important to review. Alongside it, you are shown a list of remaining functions that have not yet been reviewed. After reviewing the current function, you must decide whether to continue reviewing additional functions or conclude the review if you believe a sufficient assessment has been made.

This iterative review structure is designed to leverage your ability to maintain short-term memory context effectively. By focusing on one function at a time and assessing remaining ones, your responses provide a more focused and coherent review process than bulk analysis.

Each of your responses must follow the **strict structured format** described below. You must begin your review with a numbered severity level (1 to 3):
- 1: Minor issues or stylistic suggestions; function is safe to ship.
- 2: Functionally fine but has maintainability or clarity concerns.
- 3: Critical issues affecting performance, logic, or stability.

=== CURRENT FUNCTION FOR REVIEW ===
Below is the function you previously marked as important, along with its file path:

{}\n{}\n

=== LIST OF REMAINING FUNCTIONS ===
Here are the remaining functions that haven’t been reviewed yet. After completing the current function’s review, you must select the next function to review from this list. If you feel your review is comprehensive enough already, you may choose to stop here.

{}\n

=== RESPONSE FORMAT (STRICT) ===
Your response **must** follow this exact structure, using backticks `` as delimiters:

{{functionName~severity~functionReview~number~nextFunctionName~nextFunctionPath}}

Where:
- ``functionName``: Name of the function you're reviewing now.
- ``severity``: A numeric severity level (1 = low, 2 = moderate, 3 = high).
- ``functionReview``: A concise, professional review in 200 words or fewer, with no line breaks.
- ``number``: 1 if you want to continue reviewing more functions, 0 if you’re satisfied with your assessment.
- ``nextFunctionName``: Name of the next function you want to review (only if number is 1).
- ``nextFunctionPath``: Full path to the next function file (only if number is 1).

Example response:
{{processFile``1``This function handles file processing with good error handling``1``validateInput``src/utils.rs}}

CRITICAL RESPONSE FORMAT REQUIREMENTS:
1. Respond ONLY with this exact format: {{function_name``severity``review_text``number``next_function_name``path/to/file}}
2. Do NOT use any markdown, quotes, backticks inside the review text, or any other formatting in the response
3. Do NOT add any additional text before or after the response
4. Do NOT include any line breaks in the review text
5. The response must be a single line in the exact format shown",
    function_content,
    func.filepath,
    remaining_functions_list
);


    let response = send_prompt_to_chatgpt(&prompt).await?;
    
    let return_value = ReviewedFunction {
        name: func.name.clone(),
        filepath: func.filepath.clone(),
        start_line: func.start_line,
        end_line: func.end_line,
        review_message: response,
        function_map: remaining_functions.clone()
    };
    
    trace!("Review completed for function: {}", func.name);
    Ok(return_value)
}



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
    println!("Reviewer actor is fired up🚀");

    let mut state_guard = steady_state(&state, || FunctionreviewerInternalState::default()).await;
    if let Some(mut _state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut functions_rx = functions_rx.lock().await;
   let mut reviewed_tx = reviewed_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||functions_rx.is_closed_and_empty() && reviewed_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut functions_rx,1)    );

         match cmd.try_take(&mut functions_rx) {
            Some(rec) => {

                  let reviewed = review_function(&rec, &rec.function_map).await?;
                  
                  println!("reviewer - archive \n{:#?}", &reviewed);

                  //TODO:  here is an example writing to reviewed_tx
                  match cmd.try_send(&mut reviewed_tx, reviewed) {
                      Ok(()) => {
                          println!("Successfully sent review to archive")
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
       let _results_reviewed_vec = test_reviewed_rx.testing_take().await;
        }
}