
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;


use std::error::Error;
use crate::actor::api_response_saver::CheckForFiles;

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;



#[derive(Default)]
#[derive(Debug)]
pub(crate) struct RawFileData {
   pub data: String //TODO: : replace this and put your fields here
}




//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
struct FilereaderInternalState {
     
     
     
}
impl FilereaderInternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
           ////TODO: : add custom arg based init here
           ..Default::default()
        }
    }
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,check_for_files_rx: SteadyRx<CheckForFiles>
        ,file_content_tx: SteadyTx<RawFileData>) -> Result<(),Box<dyn Error>> {
  internal_behavior(context,check_for_files_rx,file_content_tx).await
}

//TODO : Stuck in infinite loop, make sure actor hears api_response_saver on channel
//TODO : Potential error is the handling of the response from api_response_saver 

async fn internal_behavior(context: SteadyContext
        ,check_for_files_rx: SteadyRx<CheckForFiles>
        ,file_content_tx: SteadyTx<RawFileData>) -> Result<(),Box<dyn Error>> {

    // here is how to access the CLI args if needed
    let cli_args = context.args::<Args>();

    // here is how to initialize the internal state if needed
    let mut state = if let Some(args) = cli_args {
        FilereaderInternalState::new(args)
    } else {
        FilereaderInternalState::default()
    };

    // monitor consumes context and ensures all the traffic on the passed channels is monitored
    let mut monitor =  into_monitor!(context, [check_for_files_rx],[file_content_tx]);

   //every channel must be locked before use, if this actor should panic the lock will be released
   //and the replacement actor will lock them here again
 
    let mut check_for_files_rx = check_for_files_rx.lock().await;
 
    let mut file_content_tx = file_content_tx.lock().await;
 
    let mut first = true;

    //this is the main loop of the actor, will run until shutdown is requested.
    //the closure is called upon shutdown to determine if we need to postpone the shutdown for this actor
    while monitor.is_running(&mut ||
    check_for_files_rx.is_closed_and_empty() && file_content_tx.mark_closed()) {

    if first {
        let _clean = wait_for_all!(monitor.wait_vacant_units(&mut file_content_tx,5)    );
        first = false;

    }

     //TODO:  here are all the channels you can read from
        if !first {
            let _clean = wait_for_all!(
                monitor.wait_avail_units(&mut check_for_files_rx,1), // Wait for available input
                monitor.wait_vacant_units(&mut file_content_tx,1)  // Wait for space to send AI response
            );

            let file_check = monitor.try_take(&mut check_for_files_rx).ok_or("No user input received")?;

            let check_for_files_rx_ref: &mut Rx<CheckForFiles> = &mut check_for_files_rx;
        }

     //TODO:  here are all the channels you can write to
        let file_content_tx_ref: &mut Tx<RawFileData> = &mut file_content_tx;

        let file_data = RawFileData {  
            data: readFileContents(&getFilePath(String::from("test/pythonTest.py"))).unwrap()
        };

        match monitor.try_send(&mut file_content_tx, file_data) {
            Ok(_) => print!("\nSuccessfully sent ai output.\n"),
            Err(err) => print!("\nFailed to send user input: {:?}\n", err),
        }





     //TODO:  to get started try calling the monitor.* methods:
      //    try_take<T>(&mut self, this: &mut Rx<T>) -> Option<T>  ie monitor.try_take(...
      //    try_send<T>(&mut self, this: &mut Tx<T>, msg: T) -> Result<(), T>  ie monitor.try_send(...

     monitor.relay_stats_smartly();

    }
    Ok(())
}


// Function to get the file path input from the user
fn getFilePath(dir: String) -> String {

    let mut filePath: String;

    // Trim surrounding quotes if present
    // if dir.starts_with('"') && dir.ends_with('"') {
    //     filePath = dir[1..dir.len()-1].to_string();
    // }

    let path = Path::new(&dir);

    if path.exists() {
        return dir; // Return the valid file path
    } else {
        return String::from("NO FILE PATH");
    }
}



// Function to read the content from the file
fn readFileContents(filePath: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(filePath);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}



#[cfg(test)]
pub async fn run(context: SteadyContext
        ,check_for_files_rx: SteadyRx<CheckForFiles>
        ,file_content_tx: SteadyTx<RawFileData>) -> Result<(),Box<dyn Error>> {


    let mut monitor =  into_monitor!(context, [
                            check_for_files_rx],[
                            file_content_tx]
                               );

    if let Some(responder) = monitor.sidechannel_responder() {

         
            let mut check_for_files_rx = check_for_files_rx.lock().await;
         
            let mut file_content_tx = file_content_tx.lock().await;
         

         while monitor.is_running(&mut ||
             check_for_files_rx.is_closed_and_empty() && file_content_tx.mark_closed()) {

                //TODO:  write responder code:: let responder = responder.respond_with(|message| {

                monitor.relay_stats_smartly();
         }

    }

    Ok(())

}

// *** TEST ***

// #[cfg(test)]
// pub(crate) mod tests {
//     use std::time::Duration;
//     use steady_state::*;
//     use super::*;


//     #[async_std::test]
//     pub(crate) async fn test_simple_process() {
//        let mut graph = GraphBuilder::for_testing().build(());

//        //TODO:  you may need to use .build() or  .build_as_bundle::<_, SOME_VALUE>()
//        //let (check_for_files_rx,test_check_for_files_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  you may need to use .build() or  .build_as_bundle::<_, SOME_VALUE>()
//        //let (test_file_content_rx,file_content_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  uncomment to add your test
//         //graph.actor_builder()
//         //            .with_name("UnitTest")
//         //            .build_spawn( move |context|
//         //                    internal_behavior(context,check_for_files_rx.clone(),file_content_tx.clone())
//         //             );

//         graph.start(); //startup the graph

//         //TODO:  add your test values here

//         graph.request_stop(); //our actor has no input so it immediately stops upon this request
//         graph.block_until_stopped(Duration::from_secs(15));

//         //TODO:  confirm values on the output channels
//         //    assert_eq!(XX_rx_out[0].testing_avail_units().await, 1);
//     }


// }