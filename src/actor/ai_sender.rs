use futures::FutureExt;
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use crate::actor::input_receiver::UserInput;
use std::error::Error;

use reqwest; // Add reqwest for HTTP requests
use serde_json::json; // For building JSON payloads

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
    let cli_args = context.args::<Args>();
    // let mut state = if let Some(args) = cli_args {
    //     AisenderInternalState::new(args)
    // } else {
    //     AisenderInternalState::default()
    // };

    let mut monitor = into_monitor!(context, [user_input_rx], [ai_response_tx]);

    let mut user_input_rx = user_input_rx.lock().await;
    let mut ai_response_tx = ai_response_tx.lock().await;


    while monitor.is_running(&mut || user_input_rx.is_closed_and_empty() && ai_response_tx.mark_closed()) {
        // Wait for user input
        
        // let user_input = monitor.try_peek(&mut user_input_rx);
        // println!("SENT TO API: {}", user_input.prompt);


        let _clean = wait_for_all!(
            monitor.wait_avail_units(&mut user_input_rx,1),
            monitor.wait_vacant_units(&mut ai_response_tx,1)
           );

        let user_input = monitor.try_take(&mut user_input_rx).ok_or("No user input received")?;


        // let user_input = monitor.peek_async(&mut user_input_rx);

        println!("\nAI RESPONSE: {:?}", user_input);



        // Call OpenAI API with the user input
        let ai_response = call_openai_api(&user_input.prompt).await.unwrap();
        // print!("AI RESPONSE: {}", ai_response);
        
        // Send the AI response through the channel
        let response_message = AIResponse { response_text: ai_response };   

        // println!("AI RESPONSE: {}", response_message.response_text);

        // let response_message: AIResponse = AIResponse { response_text: String::from("TEST") };   


        //commented out
        match monitor.try_send(&mut ai_response_tx, response_message) {
            Ok(_) => print!("Successfully sent user input."),
            Err(err) => print!("Failed to send user input: {:?}", err),
        }


        monitor.relay_stats_smartly(); // Relay monitoring stats
        // break;
    }
    Ok(())
}


// async fn internal_behavior(
//     context: SteadyContext,
//     user_input_rx: SteadyRx<UserInput>,  // Channel to receive user input from input_receiver
//     ai_response_tx: SteadyTx<AIResponse>, // Channel to send AI response to response_printer
// ) -> Result<(), Box<dyn Error>> {
//     let mut monitor = into_monitor!(context, [user_input_rx], [ai_response_tx]);
//     let mut user_input_rx = user_input_rx.lock().await;
//     let mut ai_response_tx = ai_response_tx.lock().await;

//     let mut buffer: [UserInput; 1000] = core::array::from_fn(|_| UserInput::default());

//     while monitor.is_running(&mut || user_input_rx.is_closed_and_empty() && ai_response_tx.mark_closed()) {
//         // Wait for user input and response availability
//         let _clean = wait_for_all!(
//             monitor.wait_avail_units(&mut user_input_rx, 1),   // Wait for at least 1 user input
//             monitor.wait_vacant_units(&mut ai_response_tx, 1)  // Wait for space in response channel
//         );

//         let count = monitor.try_peek(&mut user_input_rx);
//         if matches!(count, Some(T)){
//             // Debug: Log that we received user input
//             print!("ai_sender: Received user input");

//             //let user_input = &buffer[count - 1];  // Get the last user input from the buffer
            
//             // Call OpenAI API (simulated here for brevity)
//             let ai_response_text = format!("Response to '{}'", user_input.prompt);

//             // Debug: Log the AI response
//             print!("ai_sender: Sending AI response: {}", ai_response_text);

//             // Prepare the AI response
//             let ai_response = AIResponse { response_text: ai_response_text };

//             // Send the AI response through the channel to response_printer
//             let _ = monitor.try_send(&mut ai_response_tx, ai_response).expect("Failed to send AI response");

//             // Consume the user input from the input channel
//             monitor.take_slice(&mut user_input_rx, &mut buffer[0..count]);
            
//             monitor.relay_stats_smartly();
//         } else {
//             print!("ai_sender: No user input received");
//         }
//     }
//     Ok(())
// }






// Function to call OpenAI API and retrieve a response
async fn call_openai_api(prompt: &str) -> Result<String, Box<dyn Error>> {

    println!("Starting API Processing");
    let api_key = "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"; // Replace with your OpenAI API key
    let client = reqwest::Client::new();
    let response = client.post("https://api.openai.com/v1/completions")
        .bearer_auth(api_key)
        .json(&json!({
            "model": "text-davinci-003", // Use your desired model
            "prompt": prompt,
            "max_tokens": 100, // Adjust as needed
        }))
        .send()
        .await?;

    let response_json: serde_json::Value = response.json().await?;
    let ai_text = response_json["choices"][0]["text"].as_str().unwrap_or("").to_string(); // Get the AI's response

    println!("AI REVIEW DONE");

    Ok(ai_text)
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
