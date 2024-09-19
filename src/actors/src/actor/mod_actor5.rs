
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
use crate::actor::mod_actor4::InterpretAndSave;






//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
struct Actor5InternalState {
     
     
}
impl Actor5InternalState {
    fn new(cli_args: &Args) -> Self {
        Self {
           ////TODO: : add custom arg based init here
           ..Default::default()
        }
    }
}



#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,actor4_to_actor5_rx: SteadyRx<InterpretAndSave>) -> Result<(),Box<dyn Error>> {

    let cli_args = context.args::<Args>();
    let mut state = if let Some(args) = cli_args {
        Actor5InternalState::new(args)
    } else {
        Actor5InternalState::default()
    };

    let mut monitor =  into_monitor!(context, [
                        actor4_to_actor5_rx],[]
                           );

 let mut actor4_to_actor5_rx = actor4_to_actor5_rx.lock().await;
 

    while monitor.is_running(&mut ||
    actor4_to_actor5_rx.is_closed_and_empty()) {

         let _clean = wait_for_all!(monitor.wait_periodic(Duration::from_millis(1000))    );


     process_once(&mut monitor, &mut state
         , &mut actor4_to_actor5_rx).await;

     monitor.relay_stats_smartly();

    }
    Ok(())
}

async fn process_once<const R: usize, const T: usize>(monitor: & mut LocalMonitor<R,T>
                          , state: &mut Actor5InternalState, actor4_to_actor5_rx: &mut Rx<InterpretAndSave>
                             ) {

    //trythis:  monitor.try_take(actor4_to_actor5_rx);

    ////TODO: : put your implementation here

}

#[cfg(test)]
pub async fn run(context: SteadyContext
        ,actor4_to_actor5_rx: SteadyRx<InterpretAndSave>) -> Result<(),Box<dyn Error>> {

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
         let (actor4_to_actor5_tx_extern, actor4_to_actor5_rx) = graph.channel_builder().with_capacity(8).build();
         let mock_context = graph.new_test_monitor("mock");
         let mut mock_monitor = into_monitor!(mock_context, [], []);
         let mut actor4_to_actor5_tx_extern = actor4_to_actor5_tx_extern.lock().await;
         let mut actor4_to_actor5_rx = actor4_to_actor5_rx.lock().await;
         ////TODO: : add assignments

        process_once(&mut monitor, &mut state
                 , &mut actor4_to_actor5_rx).await;


    }
}