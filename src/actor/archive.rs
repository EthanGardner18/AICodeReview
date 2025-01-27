#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::function_reviewer::ReviewedFunction;

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct ArchivedFunction {
    pub name: String,
    pub namespace: String,
    pub filepath: String,
    pub start_line: usize,
    pub end_line: usize,
    pub review_message: String,
}

#[derive(Default,Clone,Debug,Eq,PartialEq,Copy)]
pub(crate) struct LoopSignal {
   pub state: bool //TODO:  remove dummy and put your channel message fields here
}

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ArchiveInternalState {
}


pub async fn run(context: SteadyContext
        ,reviewed_rx: SteadyRx<ReviewedFunction>
        ,loop_feedback_tx: SteadyTx<LoopSignal>
        ,archived_tx: SteadyTx<ArchivedFunction>, state: SteadyState<ArchiveInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [reviewed_rx],[loop_feedback_tx,archived_tx]);
  internal_behavior(cmd,reviewed_rx, loop_feedback_tx, archived_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,reviewed_rx: SteadyRx<ReviewedFunction>,loop_feedback_tx: SteadyTx<LoopSignal>,archived_tx: SteadyTx<ArchivedFunction>, state: SteadyState<ArchiveInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || ArchiveInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut reviewed_rx = reviewed_rx.lock().await;
   let mut loop_feedback_tx = loop_feedback_tx.lock().await;
   let mut archived_tx = archived_tx.lock().await;

   let mut loop_struct = LoopSignal {
        state: true
   };

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||reviewed_rx.is_closed_and_empty() && loop_feedback_tx.mark_closed() && archived_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut reviewed_rx,1)    );

  
          //TODO:  here is an example reading from reviewed_rx
          match cmd.try_take(&mut reviewed_rx) {
              Some(reviewed) => {
                  let archived = ArchivedFunction {
                      name: reviewed.name,
                      namespace: reviewed.namespace,
                      filepath: reviewed.filepath,
                      start_line: reviewed.start_line,
                      end_line: reviewed.end_line,
                      review_message: reviewed.review_message,
                  };
                  println!("got rec IN ARCHIVE: {:?}", archived);

                    //TODO:  here is an example writing to archived_tx
                    match cmd.try_send(&mut archived_tx, archived ) {
                        Ok(()) => {
                        },
                        Err(msg) => { //in the above await we should have confirmed space is available
                            trace!("error sending: {:?}", msg)
                        },
                    }
              }
              None => {
                  if clean {
                     //this could be an error if we expected a value
                     loop_struct.state = false

                  }
              }
          }

          
        ////   let reviewed_function  = cmd.try_take(&mut reviewed_rx).ok_or("ERROR @ archive.rs")?;
  
        //TODO:  here is an example writing to loop_feedback_tx
        match cmd.try_send(&mut loop_feedback_tx, loop_struct ) {
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
       let (test_reviewed_tx,reviewed_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (loop_feedback_tx,test_loop_feedback_rx) = graph.channel_builder().with_capacity(4).build();
       
       let (archived_tx,test_archived_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, reviewed_rx.clone(), loop_feedback_tx.clone(), archived_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_reviewed_tx.testing_send_all(vec![ReviewedFunction::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_loop_feedback_rx.testing_avail_units().await, 1); // check expected count
       let results_loop_feedback_vec = test_loop_feedback_rx.testing_take().await;
        
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_archived_rx.testing_avail_units().await, 1); // check expected count
       let results_archived_vec = test_archived_rx.testing_take().await;
        }
}