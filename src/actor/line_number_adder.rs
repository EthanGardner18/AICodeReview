
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;


use std::error::Error;
use crate::actor::file_reader::RawFileData;


#[derive(Default)]
pub(crate) struct NumberedFileData {
   dummy: u8 //TODO: : replace this and put your fields here
}




//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
struct LinenumberadderInternalState {
     
     
     
}
impl LinenumberadderInternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
           ////TODO: : add custom arg based init here
           ..Default::default()
        }
    }
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,file_content_rx: SteadyRx<RawFileData>
        ,numbered_content_tx: SteadyTx<NumberedFileData>) -> Result<(),Box<dyn Error>> {
  internal_behavior(context,file_content_rx,numbered_content_tx).await
}

async fn internal_behavior(context: SteadyContext
        ,file_content_rx: SteadyRx<RawFileData>
        ,numbered_content_tx: SteadyTx<NumberedFileData>) -> Result<(),Box<dyn Error>> {

    // here is how to access the CLI args if needed
    let cli_args = context.args::<Args>();

    // here is how to initialize the internal state if needed
    let mut state = if let Some(args) = cli_args {
        LinenumberadderInternalState::new(args)
    } else {
        LinenumberadderInternalState::default()
    };

    // monitor consumes context and ensures all the traffic on the passed channels is monitored
    let mut monitor =  into_monitor!(context, [
                        file_content_rx],[
                        numbered_content_tx]
                           );

   //every channel must be locked before use, if this actor should panic the lock will be released
   //and the replacement actor will lock them here again
 
    let mut file_content_rx = file_content_rx.lock().await;
 
    let mut numbered_content_tx = numbered_content_tx.lock().await;
 

    //this is the main loop of the actor, will run until shutdown is requested.
    //the closure is called upon shutdown to determine if we need to postpone the shutdown for this actor
    while monitor.is_running(&mut ||
    file_content_rx.is_closed_and_empty() && numbered_content_tx.mark_closed()) {

         let _clean = wait_for_all!(monitor.wait_avail_units(&mut file_content_rx,1)    );


     //TODO:  here are all the channels you can read from
          let file_content_rx_ref: &mut Rx<RawFileData> = &mut file_content_rx;

          let file_data_struct = monitor.try_take(&mut file_content_rx).ok_or("No user input received")?;

          let file_data = file_data_struct.data;
          let numbered_data = addLineNumbers(file_data);
        


     //TODO:  here are all the channels you can write to
          let numbered_content_tx_ref: &mut Tx<NumberedFileData> = &mut numbered_content_tx;

        // Try to send the AI response through the ai_response_tx channel
        match monitor.try_send(&mut ai_response_tx, response_message) {
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



// Function to add line numbers to each line of the file
fn addLineNumbers(contents: String) -> String {
    contents
        .lines()
        .enumerate()
        .map(|(i, line)| format!("{}: {}", i + 1, line))  // Line numbers start from 1
        .collect::<Vec<String>>()
        .join("\n")
}



#[cfg(test)]
pub async fn run(context: SteadyContext
        ,file_content_rx: SteadyRx<RawFileData>
        ,numbered_content_tx: SteadyTx<NumberedFileData>) -> Result<(),Box<dyn Error>> {


    let mut monitor =  into_monitor!(context, [
                            file_content_rx],[
                            numbered_content_tx]
                               );

    if let Some(responder) = monitor.sidechannel_responder() {

         
            let mut file_content_rx = file_content_rx.lock().await;
         
            let mut numbered_content_tx = numbered_content_tx.lock().await;
         

         while monitor.is_running(&mut ||
             file_content_rx.is_closed_and_empty() && numbered_content_tx.mark_closed()) {

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
//        //let (file_content_rx,test_file_content_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  you may need to use .build() or  .build_as_bundle::<_, SOME_VALUE>()
//        //let (test_numbered_content_rx,numbered_content_tx) = graph.channel_builder().with_capacity(4).build()
//        //TODO:  uncomment to add your test
//         //graph.actor_builder()
//         //            .with_name("UnitTest")
//         //            .build_spawn( move |context|
//         //                    internal_behavior(context,file_content_rx.clone(),numbered_content_tx.clone())
//         //             );

//         graph.start(); //startup the graph

//         //TODO:  add your test values here

//         graph.request_stop(); //our actor has no input so it immediately stops upon this request
//         graph.block_until_stopped(Duration::from_secs(15));

//         //TODO:  confirm values on the output channels
//         //    assert_eq!(XX_rx_out[0].testing_avail_units().await, 1);
//     }


// }