use futures::FutureExt;
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use crate::actor::input_receiver::UserInput;
use std::error::Error;

use surf::Client;
use serde_json::json;


#[derive(Default)]
#[derive(Debug)]
pub(crate) struct AIResponse {
    pub response_text: String, // Store the AI response here
}

// Internal state for the AI sender (optional)
#[derive(Default)]
struct AisenderInternalState {}

impl AisenderInternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
            // Initialize custom fields from CLI args if necessary
            ..Default::default()
        }
    }
}

#[cfg(not(test))]
pub async fn run(
    context: SteadyContext,
    user_input_rx: SteadyRx<UserInput>,
    ai_response_tx: SteadyTx<AIResponse>,
) -> Result<(), Box<dyn Error>> {
    internal_behavior(context, user_input_rx, ai_response_tx).await
}

async fn internal_behavior(
    context: SteadyContext,
    user_input_rx: SteadyRx<UserInput>,
    ai_response_tx: SteadyTx<AIResponse>,
) -> Result<(), Box<dyn Error>> {

    // Get any command-line arguments passed to the program
    let cli_args = context.args::<Args>();

    // Set the initial state of the AI sender based on the command-line arguments (if provided), or use default values
    let mut state = if let Some(args) = cli_args {
        AisenderInternalState::new(args)
    } else {
        AisenderInternalState::default()
    };

    // Create a monitor to handle channel traffic between user input and AI response
    let mut monitor = into_monitor!(context, [user_input_rx], [ai_response_tx]);

    // Lock the user input and AI response channels to ensure safe access across multiple tasks
    let mut user_input_rx = user_input_rx.lock().await;
    let mut ai_response_tx = ai_response_tx.lock().await;

    // Main loop that runs while the monitor is active
    while monitor.is_running(&mut || user_input_rx.is_closed_and_empty() && ai_response_tx.mark_closed()) {

        // Wait until both input (user_input_rx) and output (ai_response_tx) are available
        let _clean = wait_for_all!(
            monitor.wait_avail_units(&mut user_input_rx,1), // Wait for available input
            monitor.wait_vacant_units(&mut ai_response_tx,1)  // Wait for space to send AI response
        );

        // Try to take the user input from the receive channel, if none is available, return an error
        let user_input = monitor.try_take(&mut user_input_rx).ok_or("No user input received")?;

        // API key for OpenAI (this should ideally be stored securely)
        let api_key = "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";

        // Call the OpenAI API with the user input prompt and get the response
        let ai_response = call_openai_api(&user_input.prompt, api_key).await?;
        
        // Create a message to send to the next actor with the AI's response
        let response_message = AIResponse { response_text: ai_response };   

        // Try to send the AI response through the ai_response_tx channel
        match monitor.try_send(&mut ai_response_tx, response_message) {
            Ok(_) => print!("\nSuccessfully sent ai output.\n"),
            Err(err) => print!("\nFailed to send user input: {:?}\n", err),
        }

        // Relay monitoring statistics for debugging/performance analysis
        monitor.relay_stats_smartly(); // Relay monitoring stats

    }
    Ok(())
}

// Function to call the OpenAI API asynchronously and get a response
pub async fn call_openai_api(prompt: &str, api_key: &str) -> Result<String, Box<dyn Error>> {

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
pub async fn run(
    context: SteadyContext,
    user_input_rx: SteadyRx<UserInput>,
    ai_response_tx: SteadyTx<AIResponse>,
) -> Result<(), Box<dyn Error>> {
    let mut monitor = into_monitor!(context, [user_input_rx], [ai_response_tx]);

    if let Some(responder) = monitor.sidechannel_responder() {
        let mut user_input_rx = user_input_rx.lock().await;
        let mut ai_response_tx = ai_response_tx.lock().await;

        while monitor.is_running(&mut || user_input_rx.is_closed_and_empty() && ai_response_tx.mark_closed()) {
            // Responder code can be added here
            monitor.relay_stats_smartly();
        }
    }

    Ok(())
}

// *** TESTS ***

// #[cfg(test)]
// pub(crate) mod tests {
//     use std::time::Duration;
//     use steady_state::*;
//     use super::*;

//     #[async_std::test]
//     pub(crate) async fn test_simple_process() {
//         let mut graph = GraphBuilder::for_testing().build(());

//         // Create channels for testing
//         let (user_input_rx, user_input_tx) = graph.channel_builder().with_capacity(4).build();
//         let (ai_response_rx, ai_response_tx) = graph.channel_builder().with_capacity(4).build();

//         graph.actor_builder()
//             .with_name("AISender")
//             .build_spawn(move |context| internal_behavior(context, user_input_rx.clone(), ai_response_tx.clone()));

//         graph.start(); // Start the graph

//         // Simulate user input for testing
//         let test_input = UserInput { prompt: "What is the capital of France?".to_string() };
//         user_input_tx.try_send(test_input).unwrap();

//         graph.request_stop(); // Request the actor to stop
//         graph.block_until_stopped(Duration::from_secs(15));

//         // TODO: Confirm values on the output channels
//         // e.g. assert_eq!(some_condition, expected_value);
//     }
// }
