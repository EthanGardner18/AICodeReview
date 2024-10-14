#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;

use std::error::Error;

// Struct to hold user input
#[derive(Default)]
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
    let cli_args = context.args::<Args>();
    let mut state = if let Some(args) = cli_args {
        InputReceiverInternalState::new(args)
    } else {
        InputReceiverInternalState::default()
    };

    // Monitor for channel traffic
    let mut monitor = into_monitor!(context, [], [user_input_tx]);

    // Lock the channel for sending messages
    let mut user_input_tx = user_input_tx.lock().await;

    while monitor.is_running(&mut || user_input_tx.mark_closed()) {
        // Wait for user input
        let user_input = get_user_input(); // Implement this function to read user input

        // Send the user input to the next actor
        let user_input_msg = UserInput { prompt: user_input };

        //commented out
        if let Err(_) = user_input_tx.try_send(user_input_msg) {
            error!("Failed to send user input, the channel may be closed.");
        }

        monitor.relay_stats_smartly(); // Relay monitoring stats
        break;
    }
    Ok(())
}

// Dummy function to simulate user input; replace with actual input handling logic
fn get_user_input() -> String {
    use std::io::{self, Write};

    print!("Enter your input: ");
    io::stdout().flush().unwrap(); // Ensure prompt is displayed immediately

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap(); // Read user input
    input.trim().to_string() // Return trimmed input
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

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;
    use steady_state::*;
    use super::*;

    #[async_std::test]
    pub(crate) async fn test_simple_process() {
        let mut graph = GraphBuilder::for_testing().build(());

        // Create a channel for testing
        let (test_user_input_rx, user_input_tx) = graph.channel_builder().with_capacity(4).build();

        graph.actor_builder()
            .with_name("InputReceiver")
            .build_spawn(move |context| internal_behavior(context, user_input_tx.clone()));

        graph.start(); // Start the graph

        // TODO: Add your test values here
        // Example: Simulate user input or assert expected behavior

        graph.request_stop(); // Request the actor to stop
        graph.block_until_stopped(Duration::from_secs(15));

        // TODO: Confirm values on the output channels
    }
}
