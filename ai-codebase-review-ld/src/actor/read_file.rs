
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::read_directory::DirectoryPath;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct FileContent {
   _dummy: u8 //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ReadfileInternalState {
}


pub async fn run(context: SteadyContext
        ,directory_rx: SteadyRx<DirectoryPath>
        ,file_tx: SteadyTx<FileContent>, state: SteadyState<ReadfileInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [directory_rx],[file_tx]);
  internal_behavior(cmd,directory_rx, file_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,directory_rx: SteadyRx<DirectoryPath>,file_tx: SteadyTx<FileContent>, state: SteadyState<ReadfileInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || ReadfileInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut directory_rx = directory_rx.lock().await;
   let mut file_tx = file_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||directory_rx.is_closed_and_empty() && file_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut directory_rx,1)    );

  
          //TODO:  here is an example reading from directory_rx
          match cmd.try_take(&mut directory_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
        //TODO:  here is an example writing to file_tx
        match cmd.try_send(&mut file_tx, FileContent::default() ) {
            Ok(()) => {
            },
            Err(msg) => { //in the above await we should have confirmed space is available
                trace!("error sending: {:?}", msg)
            },
        }
  

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
       let (test_directory_tx,directory_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (file_tx,test_file_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, directory_rx.clone(), file_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_directory_tx.testing_send_all(vec![DirectoryPath::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_file_rx.testing_avail_units().await, 1); // check expected count
       let results_file_vec = test_file_rx.testing_take().await;
        }
}