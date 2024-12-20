
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::function_reviewer::ReviewedFunction;

#[derive(Default,Clone,Debug,Eq,PartialEq,Copy)]
pub(crate) struct ArchivedFunction {
   _dummy: u8 //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ArchiveInternalState {
}


pub async fn run<const ARCHIVED_FUNCTION_TX_GIRTH:usize,>(context: SteadyContext
        ,reviewed_function_rx: SteadyRx<ReviewedFunction>
        ,archived_function_tx: SteadyTxBundle<ArchivedFunction, ARCHIVED_FUNCTION_TX_GIRTH>, state: SteadyState<ArchiveInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [reviewed_function_rx],[archived_function_tx[0],archived_function_tx[1]]);
  internal_behavior(cmd,reviewed_function_rx, archived_function_tx, state).await
}

async fn internal_behavior<C: SteadyCommander,const ARCHIVED_FUNCTION_TX_GIRTH:usize
,>(mut cmd: C,reviewed_function_rx: SteadyRx<ReviewedFunction>,archived_function_tx: SteadyTxBundle<ArchivedFunction
, ARCHIVED_FUNCTION_TX_GIRTH>, state: SteadyState<ArchiveInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || ArchiveInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut reviewed_function_rx = reviewed_function_rx.lock().await;
   let mut archived_function_tx = archived_function_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||reviewed_function_rx.is_closed_and_empty() && archived_function_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut reviewed_function_rx,1)    );

  
          //TODO:  here is an example reading from reviewed_function_rx
          match cmd.try_take(&mut reviewed_function_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
        //TODO:  here is an example writing to one of the archived_function_tx bundle of 2
        match cmd.try_send(&mut archived_function_tx[0], ArchivedFunction::default() ) {
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
       let (test_reviewed_function_tx,reviewed_function_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (archived_function_tx,test_archived_function_rx) = graph.channel_builder().with_capacity(4).build_as_bundle::<_, 2>();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, reviewed_function_rx.clone(), archived_function_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_reviewed_function_tx.testing_send_all(vec![ReviewedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels, further this is only one in the bundle
       //    assert_eq!(test_archived_function_rx[0].testing_avail_units().await, 1); // check expected count
       let results_archived_function_vec = test_archived_function_rx[0].testing_take().await;
        }
}