
#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use steady_state::monitor::LocalMonitor;
use crate::Args;
use futures::join;
use futures::select;

use std::error::Error;


#[derive(Default)]
pub(crate) struct EstablishConnection {
   //TODO: : add your fields here
}




//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
struct Actor1InternalState {
     
     
     
}
impl Actor1InternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
           ////TODO: : add custom arg based init here
           ..Default::default()
        }
    }
}



#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,actor1_to_actor3_tx: SteadyTx<EstablishConnection>) -> Result<(),Box<dyn Error>> {

    let cli_args = context.args::<Args>();
    let mut state = if let Some(args) = cli_args {
        Actor1InternalState::new(args)
    } else {
        Actor1InternalState::default()
    };

    let mut monitor =  into_monitor!(context, [],[
                        actor1_to_actor3_tx]
                           );

 
    let mut actor1_to_actor3_tx = actor1_to_actor3_tx.lock().await;

    while monitor.is_running(&mut ||actor1_to_actor3_tx.mark_closed()) {

         let _clean = wait_for_all!(monitor.wait_periodic(Duration::from_millis(1000))    );


     process_once(&mut monitor, &mut state
         , &mut actor1_to_actor3_tx).await;

     monitor.relay_stats_smartly();

    }
    Ok(())
}

async fn process_once<const R: usize, const T: usize>(monitor: & mut LocalMonitor<R,T>
                          , state: &mut Actor1InternalState, actor1_to_actor3_tx: &mut Tx<EstablishConnection>
                             ) {

    //trythis:  monitor.try_send(actor1_to_actor3_tx, send owner);

    ////TODO: : put your implementation here

}

#[cfg(test)]
pub async fn run(context: SteadyContext
        ,actor1_to_actor3_tx: SteadyTx<EstablishConnection>) -> Result<(),Box<dyn Error>> {

}

#[cfg(test)]
mod tests {
    use async_std::test;
    use steady_state::*;


    #[test]
    async fn test_process() {
        util::logger::initialize();
        let mut graph = Graph::new(());

        //build your channels as needed for testing
        let (tx, rx) = graph.channel_builder().with_capacity(8).build();
         let (actor1_to_actor3_tx,actor1_to_actor3_rx_extern) = graph.channel_builder().with_capacity(8).build();
         let mock_context = graph.new_test_monitor("mock");
         let mut mock_monitor = into_monitor!(mock_context, [], []);
         let mut actor1_to_actor3_tx = actor1_to_actor3_tx.lock().await;
         let mut actor1_to_actor3_rx_extern = actor1_to_actor3_rx_extern.lock().await;
         ////TODO: : add assignments

        process_once(&mut monitor, &mut state
                 , &mut actor1_to_actor3_tx).await;


    }
}