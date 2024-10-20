#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
    
use std::error::Error;

// Struct to hold user input
#[derive(Default)]
#[derive(Debug)]
pub(crate) struct UserInput {
    pub prompt: String, // User input prompt
}

// Internal state (optional, can be removed if not needed)
#[derive(Default)]
struct InputReceiverInternalState {
    // Add custom fields here if needed
}

impl InputReceiverInternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
            // Initialize custom fields from CLI args if necessary
            ..Default::default()
        }
    }
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext, user_input_tx: SteadyTx<UserInput>) -> Result<(), Box<dyn Error>> {
    internal_behavior(context, user_input_tx).await
}

async fn internal_behavior(context: SteadyContext, user_input_tx: SteadyTx<UserInput>) -> Result<(), Box<dyn Error>> {
    // Retrieve command-line arguments (if any) and create an initial state for the input receiver
    let cli_args = context.args::<Args>();


    let mut state = if let Some(args) = cli_args {
        // If arguments are provided, create a new state based on those arguments
        InputReceiverInternalState::new(args)
    } else {
        // Otherwise, use the default state
        InputReceiverInternalState::default()
    };

    // Set up a monitor to watch for channel activity (user_input_tx) from within the context
    let mut monitor = into_monitor!(context, [], [user_input_tx]);

    // Lock the user_input_tx channel for sending messages (owned lock for full access)
    let mut user_input_tx = user_input_tx.lock_owned().await;

    // Loop to keep processing while the monitor is running
    while monitor.is_running(&mut || user_input_tx.mark_closed()) {

        // Wait for user input (this is an asynchronous call to get input from the user)
        let user_input = get_user_input().await; // Implement this function to read user input

        // Create a message to be sent to the next actor using the user's input
        let user_input_msg = UserInput { prompt: user_input };
        
        // Try sending the message through the user_input_tx channel
        match monitor.try_send(&mut user_input_tx, user_input_msg) {
            // If sending is successful, print a success message
            Ok(_) => print!("\nSuccessfully sent user input.\n"),
            // If sending fails, print an error message with the error details
            Err(err) => print!("\nFailed to send user input: {:?}\n", err),
        }

        // Relay the monitoring stats in a smart way (for performance tracking)
        monitor.relay_stats_smartly(); // Relay monitoring stats
    }
    Ok(())
}

async fn get_user_input() -> String {
    // Import necessary modules for input/output
    use std::io::{self, Write};

    print!("Enter your input: ");
    // Flush the output to make sure the prompt is displayed immediately
    io::stdout().flush().unwrap(); 

    // Create a mutable string to hold the input
    let mut input = String::new();

    // Read the user's input from the standard input (stdin)
    io::stdin().read_line(&mut input).unwrap(); 

    // Remove any extra whitespace and return the cleaned-up input as a string
    input.trim().to_string() 
}


#[cfg(test)]
pub async fn run(context: SteadyContext, user_input_tx: SteadyTx<UserInput>) -> Result<(), Box<dyn Error>> {
    // Testing logic can be implemented here
    let mut monitor = into_monitor!(context, [], [user_input_tx]);

    if let Some(responder) = monitor.sidechannel_responder() {
        let mut user_input_tx = user_input_tx.lock().await;

        while monitor.is_running(&mut || user_input_tx.mark_closed()) {
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

//         // Create a channel for testing
//         let (test_user_input_rx, user_input_tx) = graph.channel_builder().with_capacity(4).build();

//         graph.actor_builder()
//             .with_name("InputReceiver")
//             .build_spawn(move |context| internal_behavior(context, user_input_tx.clone()));

//         graph.start(); // Start the graph

//         // TODO: Add your test values here
//         // Example: Simulate user input or assert expected behavior

//         graph.request_stop(); // Request the actor to stop
//         graph.block_until_stopped(Duration::from_secs(15));

//         // TODO: Confirm values on the output channels
//     }
// }
