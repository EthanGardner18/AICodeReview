#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::ai_sender::AIResponse;
use std::fs::OpenOptions; // For file operations
use std::io::Write; // For writing to files

// Internal state for the response printer (optional)
#[derive(Default)]
struct ResponseprinterInternalState {}

impl ResponseprinterInternalState {
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
    ai_response_rx: SteadyRx<AIResponse>,
) -> Result<(), Box<dyn Error>> {
    internal_behavior(context, ai_response_rx).await
}

async fn internal_behavior(
    context: SteadyContext,
    ai_response_rx: SteadyRx<AIResponse>,
) -> Result<(), Box<dyn Error>> {

    // Get any command-line arguments passed to the program
    let cli_args = context.args::<Args>();

    // Initialize the internal state of the ResponsePrinter based on the command-line arguments (if provided),
    // or use default values if no arguments are passed.
    let mut state = if let Some(args) = cli_args {
        ResponseprinterInternalState::new(args)
    } else {
        ResponseprinterInternalState::default()
    };

    // Create a monitor to handle incoming AI responses
    let mut monitor = into_monitor!(context, [ai_response_rx], []);

    // Lock the AI response channel to ensure safe access
    let mut ai_response_rx = ai_response_rx.lock().await;

    // Main loop that runs while the monitor is active
    while monitor.is_running(&mut || ai_response_rx.is_closed_and_empty()) {

        // Wait until there is at least 1 AI response available to process
        let _clean = wait_for_all!(monitor.wait_avail_units(&mut ai_response_rx, 1));

        // Try to take an AI response from the receive channel
        let ai_response = monitor.try_take(&mut ai_response_rx).ok_or("No AI response received")?;

        // Save the AI response to a file by calling the save_response_to_file function
        let _ = save_response_to_file(&ai_response.response_text).await;

        // Relay monitoring statistics (for debugging or performance tracking)
        monitor.relay_stats_smartly();
    }
    Ok(())
}


// Function to save the AI response to a text file
async fn save_response_to_file(response_text: &str) -> Result<(), Box<dyn Error>> {

    let file_path = "final.txt"; // Define the file path where responses will be saved

    // Open the file in append mode, create it if it doesn't exist
    let mut file = OpenOptions::new()
        .create(true) // Create the file if it doesn't exist
        .append(true) // Append to the file
        .open(file_path)?;

     // Write the AI response to the file, followed by a newline
    writeln!(file, "{}", response_text)?;
    
    Ok(())
}

#[cfg(test)]
pub async fn run(
    context: SteadyContext,
    ai_response_rx: SteadyRx<AIResponse>,
) -> Result<(), Box<dyn Error>> {
    let mut monitor = into_monitor!(context, [ai_response_rx], []);

    if let Some(responder) = monitor.sidechannel_responder() {
        let mut ai_response_rx = ai_response_rx.lock().await;

        while monitor.is_running(&mut || ai_response_rx.is_closed_and_empty()) {
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
//         let (ai_response_rx, ai_response_tx) = graph.channel_builder().with_capacity(4).build();

//         graph.actor_builder()
//             .with_name("ResponsePrinter")
//             .build_spawn(move |context| internal_behavior(context, ai_response_rx.clone()));

//         graph.start(); // Start the graph

//         // Simulate sending a response for testing
//         let test_response = AIResponse { response_text: "This is a test response.".to_string() };
//         ai_response_tx.try_send(test_response).unwrap();

//         graph.request_stop(); // Request the actor to stop
//         graph.block_until_stopped(Duration::from_secs(15));

//         // TODO: Confirm values on the output channels or file contents if necessary
//         // For instance, you could read back from `final.txt` to verify contents.
//     }
// }
