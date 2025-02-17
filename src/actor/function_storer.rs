
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::archive::ArchivedFunction;


//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct FunctionstorerInternalState {
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [archived_rx],[]);
  internal_behavior(cmd,archived_rx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || FunctionstorerInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut archived_rx = archived_rx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||archived_rx.is_closed_and_empty()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut archived_rx,1)    );

  
          //TODO:  here is an example reading from archived_rx
          match cmd.try_take(&mut archived_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                  }
              }
          }
  

      }
    }
    Ok(())
}


#[cfg(test)]
pub async fn run(context: SteadyContext
        ,archived_rx: SteadyRx<ArchivedFunction>, state: SteadyState<FunctionstorerInternalState>
    ) -> Result<(),Box<dyn Error>> {
    let mut cmd =  into_monitor!(context, [archived_rx],[]);
    if let Some(responder) = cmd.sidechannel_responder() {
         let mut archived_rx = archived_rx.lock().await;
         while cmd.is_running(&mut ||
             archived_rx.is_closed_and_empty()) {
                // in main use graph.sidechannel_director node_call(msg,"FunctionStorer")
                let _did_check = responder.equals_responder(&mut cmd,&mut archived_rx).await;
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
       let (test_archived_tx,archived_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, archived_rx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_archived_tx.testing_send_all(vec![ArchivedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));}
}