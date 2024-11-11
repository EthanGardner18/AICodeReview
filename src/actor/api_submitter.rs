
use futures::TryFutureExt;
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;


use std::error::Error;
use crate::actor::file_saver::SavedFileData;

use surf::Client;
use serde_json::json;

#[derive(Debug)]


#[derive(Default)]
pub(crate) struct ApiResponseData {
   response_data: String //TODO: : replace this and put your fields here
}




//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
struct ApisubmitterInternalState {
     
     
     
}
impl ApisubmitterInternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
           ////TODO: : add custom arg based init here
           ..Default::default()
        }
    }
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,saved_file_rx: SteadyRx<SavedFileData>
        ,api_response_tx: SteadyTx<ApiResponseData>) -> Result<(),Box<dyn Error>> {
  internal_behavior(context,saved_file_rx,api_response_tx).await
}

async fn internal_behavior(context: SteadyContext
        ,saved_file_rx: SteadyRx<SavedFileData>
        ,api_response_tx: SteadyTx<ApiResponseData>) -> Result<(),Box<dyn Error>> {

    // here is how to access the CLI args if needed
    let cli_args = context.args::<Args>();

    // here is how to initialize the internal state if needed
    let mut state = if let Some(args) = cli_args {
        ApisubmitterInternalState::new(args)
    } else {
        ApisubmitterInternalState::default()
    };

    // monitor consumes context and ensures all the traffic on the passed channels is monitored
    let mut monitor =  into_monitor!(context, [
                        saved_file_rx],[
                        api_response_tx]
                           );

   //every channel must be locked before use, if this actor should panic the lock will be released
   //and the replacement actor will lock them here again
 
    let mut saved_file_rx = saved_file_rx.lock().await;
 
    let mut api_response_tx = api_response_tx.lock().await;
 

    //this is the main loop of the actor, will run until shutdown is requested.
    //the closure is called upon shutdown to determine if we need to postpone the shutdown for this actor
    while monitor.is_running(&mut ||
    saved_file_rx.is_closed_and_empty() && api_response_tx.mark_closed()) {

         let _clean = wait_for_all!(monitor.wait_avail_units(&mut saved_file_rx,1)    );


     //TODO:  here are all the channels you can read from
          let saved_file_rx_ref: &mut Rx<SavedFileData> = &mut saved_file_rx;

     //TODO:  here are all the channels you can write to
          let api_response_tx_ref: &mut Tx<ApiResponseData> = &mut api_response_tx;

          let response = ApiResponseData {
            response_data: call_openai_api(
                "You will recieve a file of any coding language, the first line will have the path to the file you are looking at. I would like you to parse the code and only store a header for each function in this format. One issue you need to check for is that there are comments in the code, so you need to make sure you are starting at the correct line number and ending at the correct line number. Don't forget that different coding languages use different methods to comment things in and out. Also if you see a new line asssume it counts toward the total line number count. Finally if the function is within a class, give the class name:function name:

                        {Function Name, Path, Starting Line Number, Last Line Number}
If function is within a class
{dataGen:load_data, /functions/main.py, 6, 14}
{load_data, /functions/main.py, 6, 14}", 
            "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"
            ).await.unwrap()
          };

     //TODO:  to get started try calling the monitor.* methods:
      //    try_take<T>(&mut self, this: &mut Rx<T>) -> Option<T>  ie monitor.try_take(...
      //    try_send<T>(&mut self, this: &mut Tx<T>, msg: T) -> Result<(), T>  ie monitor.try_send(...


      match monitor.try_send(&mut api_response_tx, response) {
        Ok(_) => print!("\nSuccessfully sent ai output.\n"),
        Err(err) => print!("\nFailed to send user input: {:?}\n", err),
    }

     monitor.relay_stats_smartly();

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
pub async fn run(context: SteadyContext
        ,saved_file_rx: SteadyRx<SavedFileData>
        ,api_response_tx: SteadyTx<ApiResponseData>) -> Result<(),Box<dyn Error>> {


    let mut monitor =  into_monitor!(context, [
                            saved_file_rx],[
                            api_response_tx]
                               );

    if let Some(responder) = monitor.sidechannel_responder() {

         
            let mut saved_file_rx = saved_file_rx.lock().await;
         
            let mut api_response_tx = api_response_tx.lock().await;
         

         while monitor.is_running(&mut ||
             saved_file_rx.is_closed_and_empty() && api_response_tx.mark_closed()) {

                //TODO:  write responder code:: let responder = responder.respond_with(|message| {

                monitor.relay_stats_smartly();
         }

    }

    Ok(())

}

// #[cfg(test)]
// pub(crate) mod tests {
//     use std::time::Duration;
//     use steady_state::*;
//     use super::*;


//     #[async_std::test]
//     pub(crate) async fn test_simple_process() {
//        let mut graph = GraphBuilder::for_testing().build(());

//        //TODO:  you may need to use .build() or  .build_as_bundle::<_, SOME_VALUE>()
//        //let (saved_file_rx,test_saved_file_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  you may need to use .build() or  .build_as_bundle::<_, SOME_VALUE>()
//        //let (test_api_response_rx,api_response_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  uncomment to add your test
//         //graph.actor_builder()
//         //            .with_name("UnitTest")
//         //            .build_spawn( move |context|
//         //                    internal_behavior(context,saved_file_rx.clone(),api_response_tx.clone())
//         //             );

//         graph.start(); //startup the graph

//         //TODO:  add your test values here

//         graph.request_stop(); //our actor has no input so it immediately stops upon this request
//         graph.block_until_stopped(Duration::from_secs(15));

//         //TODO:  confirm values on the output channels
//         //    assert_eq!(XX_rx_out[0].testing_avail_units().await, 1);
//     }


// }