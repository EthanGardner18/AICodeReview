#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::read_file::FileContent;
use serde_json::Value as JsonValue;
use std::fs::OpenOptions;
use std::io::Write;
use async_std::fs as async_fs;
use async_std::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::json;

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ParsefunctionInternalState {
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
                 ,file_content_rx: SteadyRx<FileContent>, state: SteadyState<ParsefunctionInternalState>
) -> Result<(),Box<dyn Error>> {

	// if needed CLI Args can be pulled into state from _cli_args
	let _cli_args = context.args::<Args>();
	// monitor consumes context and ensures all the traffic on the chosen channels is monitored
	// monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
	let cmd =  into_monitor!(context, [file_content_rx],[]);
	internal_behavior(cmd,file_content_rx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,file_content_rx: SteadyRx<FileContent>, state: SteadyState<ParsefunctionInternalState>
) -> Result<(),Box<dyn Error>> {

	let mut state_guard = steady_state(&state, || ParsefunctionInternalState::default()).await;
	if let Some(mut state) = state_guard.as_mut() {

		//every read and write channel must be locked for this instance use, this is outside before the loop
		let mut file_content_rx = file_content_rx.lock().await;

		//this is the main loop of the actor, will run until shutdown is requested.
		//the closure is called upon shutdown to determine if we need to postpone the shutdown
		while cmd.is_running(&mut ||file_content_rx.is_closed_and_empty()) {

			// our loop avoids spinning by using await here on multiple criteria. clean is false if await
			// returned early due to a shutdown request or closed channel.
			let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut file_content_rx,1)    );


			//TODO:  here is an example reading from file_content_rx

			match cmd.try_take(&mut file_content_rx) {
				Some(rec) => {
					trace!("got rec: {:?}", rec);

					// Iterate through HashMap
					for (file_path, file_content) in rec.directory_files.iter() {
						let file_path_str = file_path.to_string_lossy().to_string(); // Convert PathBuf to String
						let file_content_str = file_content.as_str(); // Get reference to String content

						// Call ChatGPT API
						match call_chatgpt_api(file_content_str, &file_path_str).await {
							Ok(response) => {
								// Convert response to a formatted string
								let response_str = format!(
								    "File: {}\nResponse: {}\n\n",
								    file_path_str,
								    response
								);

								// Append the response to test.txt
								if let Err(e) = append_to_file("test.txt", &response_str) {
									eprintln!("Failed to write to file: {}", e);
								}
							}
							Err(e) => {
								eprintln!("API call failed for {}: {}", file_path_str, e);
							}
						}
					}
				}
				None => {
					if clean {
					// this could be an error if we expected a value
				}
			}
		}





	} //end bracket
}
Ok(())
}


fn append_to_file(file_name: &str, content: &str) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_name)?;
    writeln!(file, "{}", content)?;
    Ok(())
}


/// Calls OpenAI ChatGPT API for parsing functions in a given file.
async fn call_chatgpt_api(file_content: &str, file_path: &str) -> Result<JsonValue, Box<dyn Error>> {
    dotenv::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");

    let api_url = "https://api.openai.com/v1/chat/completions";

    let prompt_template = format!(
        "
        You will receive a file of any coding language, the first line will have the path to the file you are looking at. I would like you to parse the code and only store a header for each function in this format. One \
        issue you need to check for is that there are comments in the code, so you need to make sure you are starting at the correct line number and ending at the correct line number. Don't forget that different coding \
        languages use different methods to comment things in and out. Also if you see a new line assume it counts toward the total line number count. Finally, if the function is within a class, give the class \
        name:function name.\n\nFor a function within a class:\n{{class_name:function_name, path, starting_line_number, last_line_number}}\n\nFor a function without a class:\n{{function_name, path, starting_line_number, last_line_number}}
        "
    );

    let client = surf::Client::new();
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            { "role": "system", "content": "You are a code parser specializing in analyzing functions." },
            { "role": "user", "content": format!("{}\n\n{}", file_path, file_content) }
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



#[cfg(test)]
pub async fn run(context: SteadyContext
                 ,file_content_rx: SteadyRx<FileContent>, state: SteadyState<ParsefunctionInternalState>
) -> Result<(),Box<dyn Error>> {
	let mut cmd =  into_monitor!(context, [file_content_rx],[]);
	if let Some(responder) = cmd.sidechannel_responder() {
		let mut file_content_rx = file_content_rx.lock().await;
		while cmd.is_running(&mut ||
		                     file_content_rx.is_closed_and_empty()) {
			// in main use graph.sidechannel_director node_call(msg,"ParseFunction")
			let _did_check = responder.equals_responder(&mut cmd,&mut file_content_rx).await;
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
		let (test_file_content_tx,file_content_rx) = graph.channel_builder().with_capacity(4).build();
		let state = new_state();
		graph.actor_builder()
		.with_name("UnitTest")
		.build_spawn( move |context|
		              internal_behavior(context, file_content_rx.clone(), state.clone())
		            );

		graph.start(); //startup the graph
		//TODO:  adjust this vec content to make a valid test
		test_file_content_tx.testing_send_all(vec![FileContent::default()],true).await;


		graph.request_stop();
		graph.block_until_stopped(Duration::from_secs(15));
	}
}
