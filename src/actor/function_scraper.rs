
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::LoopSignal;
use crate::actor::parse_function::ParsedCode;

#[derive(Default,Clone,Debug,Eq,PartialEq,Copy)]
pub(crate) struct CodeFunction {
   _dummy: u8 //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionscraperInternalState {
}


pub async fn run(context: SteadyContext
        ,loop_feedback_rx: SteadyRx<LoopSignal>
        ,parsed_code_rx: SteadyRx<ParsedCode>
        ,functions_tx: SteadyTx<CodeFunction>, state: SteadyState<FunctionscraperInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [loop_feedback_rx,parsed_code_rx],[functions_tx]);
  internal_behavior(cmd,loop_feedback_rx,parsed_code_rx, functions_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,loop_feedback_rx: SteadyRx<LoopSignal>,parsed_code_rx: SteadyRx<ParsedCode>,functions_tx: SteadyTx<CodeFunction>, state: SteadyState<FunctionscraperInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionscraperInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut loop_feedback_rx = loop_feedback_rx.lock().await;
   let mut parsed_code_rx = parsed_code_rx.lock().await;
   let mut functions_tx = functions_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||loop_feedback_rx.is_closed_and_empty() && parsed_code_rx.is_closed_and_empty() && functions_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut parsed_code_rx,1)    );

  
          //TODO:  here is an example reading from loop_feedback_rx
          match cmd.try_take(&mut loop_feedback_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
          //TODO:  here is an example reading from parsed_code_rx
          match cmd.try_take(&mut parsed_code_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
        //TODO:  here is an example writing to functions_tx
        match cmd.try_send(&mut functions_tx, CodeFunction::default() ) {
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
       let (test_loop_feedback_tx,loop_feedback_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (test_parsed_code_tx,parsed_code_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (functions_tx,test_functions_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, loop_feedback_rx.clone(), parsed_code_rx.clone(), functions_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_loop_feedback_tx.testing_send_all(vec![LoopSignal::default()],true).await;

        
       //TODO:  adjust this vec content to make a valid test
       test_parsed_code_tx.testing_send_all(vec![ParsedCode::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_functions_rx.testing_avail_units().await, 1); // check expected count
       let results_functions_vec = test_functions_rx.testing_take().await;
        }
}