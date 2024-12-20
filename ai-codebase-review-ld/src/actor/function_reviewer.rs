
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::function_scraper::ScrapedFunction;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ReviewedFunction {
   _dummy: u8 //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionreviewerInternalState {
}


pub async fn run(context: SteadyContext
        ,scraped_function_rx: SteadyRx<ScrapedFunction>
        ,reviewed_function_tx: SteadyTx<ReviewedFunction>, state: SteadyState<FunctionreviewerInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [scraped_function_rx],[reviewed_function_tx]);
  internal_behavior(cmd,scraped_function_rx, reviewed_function_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,scraped_function_rx: SteadyRx<ScrapedFunction>,reviewed_function_tx: SteadyTx<ReviewedFunction>, state: SteadyState<FunctionreviewerInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionreviewerInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut scraped_function_rx = scraped_function_rx.lock().await;
   let mut reviewed_function_tx = reviewed_function_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||scraped_function_rx.is_closed_and_empty() && reviewed_function_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut scraped_function_rx,1)    );

  
          //TODO:  here is an example reading from scraped_function_rx
          match cmd.try_take(&mut scraped_function_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  
  
        //TODO:  here is an example writing to reviewed_function_tx
        match cmd.try_send(&mut reviewed_function_tx, ReviewedFunction::default() ) {
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
       let (test_scraped_function_tx,scraped_function_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (reviewed_function_tx,test_reviewed_function_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, scraped_function_rx.clone(), reviewed_function_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_scraped_function_tx.testing_send_all(vec![ScrapedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_reviewed_function_rx.testing_avail_units().await, 1); // check expected count
       let results_reviewed_function_vec = test_reviewed_function_rx.testing_take().await;
        }
}