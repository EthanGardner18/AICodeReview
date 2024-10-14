mod args;
use structopt::StructOpt;
#[allow(unused_imports)]
use log::*;
use crate::args::Args;
use std::time::Duration;
use steady_state::*;





mod actor {
    
        pub mod ai_sender;
    
        pub mod input_receiver;
    
        pub mod response_printer;
    
}

fn main() {
    let opt = Args::from_args();
    if let Err(e) = steady_state::init_logging(&opt.loglevel) {
        //do not use logger to report logger could not start
        eprint!("Warning: Logger initialization failed with {:?}. There will be no logging.", e);
    }

    let service_executable_name = "llm_actors";
    let service_user = "llm_actors_user";
    let systemd_command = SystemdBuilder::process_systemd_commands(  opt.systemd_action()
                                                   , opt.to_cli_string(service_executable_name)
                                                   , service_executable_name
                                                   , service_user);

    if !systemd_command {
        info!("Starting up");
        let mut graph = build_graph(steady_state::GraphBuilder::for_production().build(opt.clone()) );
        graph.start();

        {  //remove this block to run forever.
           std::thread::sleep(Duration::from_secs(60));
           graph.request_stop(); //actors can also call stop as desired on the context or monitor
        }

        graph.block_until_stopped(Duration::from_secs(2));
    }
}

fn build_graph(mut graph: Graph) -> steady_state::Graph {

    //this common root of the channel builder allows for common config of all channels
    let base_channel_builder = graph.channel_builder()
        .with_compute_refresh_window_floor(Duration::from_secs(1),Duration::from_secs(10))
        .with_type()
        .with_line_expansion(1.0f32);
    //this common root of the actor builder allows for common config of all actors
    let base_actor_builder = graph.actor_builder() //with default OneForOne supervisor
        .with_mcpu_percentile(Percentile::p80())
        .with_work_percentile(Percentile::p80())
        .with_compute_refresh_window_floor(Duration::from_secs(1),Duration::from_secs(10));
    //build channels
    
    let (aisender_ai_response_tx, responseprinter_ai_response_rx) = base_channel_builder
        .with_capacity(10)
        .build();
    
    let (inputreceiver_user_input_tx, aisender_user_input_rx) = base_channel_builder
        .with_capacity(10)
        .build();
    
    //build actors
    
    {
      
    
       base_actor_builder.with_name("AiSender")
                 .build_spawn( move |context| actor::ai_sender::run(context
                                            , aisender_user_input_rx.clone()
                                            , aisender_ai_response_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("InputReceiver")
                 .build_spawn( move |context| actor::input_receiver::run(context
                                            , inputreceiver_user_input_tx.clone())
                 );
    }
    {
       base_actor_builder.with_name("ResponsePrinter")
                 .build_spawn( move |context| actor::response_printer::run(context
                                            , responseprinter_ai_response_rx.clone())
                 );
    }
    graph
}

#[cfg(test)]
mod graph_tests {
    use async_std::test;
    use steady_state::*;
    use std::time::Duration;
    use crate::args::Args;
    use crate::build_graph;
    use std::ops::DerefMut;


    #[test]
    async fn test_graph_one() {

            let test_ops = Args {
                loglevel: "debug".to_string(),
                systemd_install: false,
                systemd_uninstall: false,
            };
            let mut graph = build_graph( GraphBuilder::for_testing().build(test_ops.clone()) );
            graph.start();
            let mut guard = graph.sidechannel_director().await;
            let g = guard.deref_mut();
            assert!(g.is_some(), "Internal error, this is a test so this back channel should have been created already");
            if let Some(plane) = g {

              //  write your test here, send messages to edge nodes and get responses
              //  let response = plane.node_call(Box::new(SOME_STRUCT), "SOME_NODE_NAME").await;
              //  if let Some(msg) = response {
              //  }

            }
            drop(guard);
            graph.request_stop();
            graph.block_until_stopped(Duration::from_secs(3));

    }
}