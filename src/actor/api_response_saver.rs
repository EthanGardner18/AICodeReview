
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;


use std::error::Error;
use crate::actor::api_submitter::ApiResponseData;

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug)]


#[derive(Default)]
pub(crate) struct CheckForFiles {
   writeState: bool //TODO: : replace this and put your fields here
}




//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
struct ApiresponsesaverInternalState {
     
     
     
}
impl ApiresponsesaverInternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
           ////TODO: : add custom arg based init here
           ..Default::default()
        }
    }
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,api_response_rx: SteadyRx<ApiResponseData>
        ,check_for_files_tx: SteadyTx<CheckForFiles>) -> Result<(),Box<dyn Error>> {
  internal_behavior(context,api_response_rx,check_for_files_tx).await
}

// TODO: Code Stuck in infinite loop, make sure that api_response_saver is sending

async fn internal_behavior(context: SteadyContext
        ,api_response_rx: SteadyRx<ApiResponseData>
        ,check_for_files_tx: SteadyTx<CheckForFiles>) -> Result<(),Box<dyn Error>> {

    // here is how to access the CLI args if needed
    let cli_args = context.args::<Args>();

    // here is how to initialize the internal state if needed
    let mut state = if let Some(args) = cli_args {
        ApiresponsesaverInternalState::new(args)
    } else {
        ApiresponsesaverInternalState::default()
    };

    // monitor consumes context and ensures all the traffic on the passed channels is monitored
    let mut monitor =  into_monitor!(context, [
                        api_response_rx],[
                        check_for_files_tx]
                           );

   //every channel must be locked before use, if this actor should panic the lock will be released
   //and the replacement actor will lock them here again
 
    let mut api_response_rx = api_response_rx.lock().await;
 
    let mut check_for_files_tx = check_for_files_tx.lock().await;
 

    //this is the main loop of the actor, will run until shutdown is requested.
    //the closure is called upon shutdown to determine if we need to postpone the shutdown for this actor
    while monitor.is_running(&mut ||
    api_response_rx.is_closed_and_empty() && check_for_files_tx.mark_closed()) {

         let _clean = wait_for_all!(monitor.wait_avail_units(&mut api_response_rx,1)    );


     //TODO:  here are all the channels you can read from
          let api_response_rx_ref: &mut Rx<ApiResponseData> = &mut api_response_rx;

     //TODO:  here are all the channels you can write to
          let check_for_files_tx_ref: &mut Tx<CheckForFiles> = &mut check_for_files_tx;

     //TODO:  to get started try calling the monitor.* methods:
      //    try_take<T>(&mut self, this: &mut Rx<T>) -> Option<T>  ie monitor.try_take(...
      //    try_send<T>(&mut self, this: &mut Tx<T>, msg: T) -> Result<(), T>  ie monitor.try_send(...




      let check = CheckForFiles {
        writeState: true
      };

      match monitor.try_send(&mut check_for_files_tx, check) {
        Ok(_) => print!("\nSuccessfully sent ai output.\n"),
        Err(err) => print!("\nFailed to send user input: {:?}\n", err),
    }
     
     
     
     monitor.relay_stats_smartly();


    }
    Ok(())
}



// Function to write the response to a file
fn writeToFile(contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut outputFile = File::create("codeReview.txt")?;
    outputFile.write_all(contents.as_bytes())?;
    println!("Code review saved to codeReview.txt.");
    Ok(())
}



#[cfg(test)]
pub async fn run(context: SteadyContext
        ,api_response_rx: SteadyRx<ApiResponseData>
        ,check_for_files_tx: SteadyTx<CheckForFiles>) -> Result<(),Box<dyn Error>> {


    let mut monitor =  into_monitor!(context, [
                            api_response_rx],[
                            check_for_files_tx]
                               );

    if let Some(responder) = monitor.sidechannel_responder() {

         
            let mut api_response_rx = api_response_rx.lock().await;
         
            let mut check_for_files_tx = check_for_files_tx.lock().await;
         

         while monitor.is_running(&mut ||
             api_response_rx.is_closed_and_empty() && check_for_files_tx.mark_closed()) {

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
//        //let (api_response_rx,test_api_response_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  you may need to use .build() or  .build_as_bundle::<_, SOME_VALUE>()
//        //let (test_check_for_files_rx,check_for_files_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  uncomment to add your test
//         //graph.actor_builder()
//         //            .with_name("UnitTest")
//         //            .build_spawn( move |context|
//         //                    internal_behavior(context,api_response_rx.clone(),check_for_files_tx.clone())
//         //             );

//         graph.start(); //startup the graph

//         //TODO:  add your test values here

//         graph.request_stop(); //our actor has no input so it immediately stops upon this request
//         graph.block_until_stopped(Duration::from_secs(15));

//         //TODO:  confirm values on the output channels
//         //    assert_eq!(XX_rx_out[0].testing_avail_units().await, 1);
//     }


// }