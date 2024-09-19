mod args;
use structopt::StructOpt;
#[allow(unused_imports)]
use log::*;
use crate::args::Args;
use std::time::Duration;
use steady_state::*;





mod actor {
    
        pub mod mod_actor1;
    
        pub mod mod_actor2;
    
        pub mod mod_actor3;
    
        pub mod mod_actor4;
    
        pub mod mod_actor5;
    
}

fn main() {
    let opt = Args::from_args();
    if let Err(e) = steady_state::init_logging(&opt.loglevel) {
        //do not use logger to report logger could not start
        eprint!("Warning: Logger initialization failed with {:?}. There will be no logging.", e);
    }

    let service_executable_name = "actors";
    let service_user = "actors_user";
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
    
    let (actor1_actor1_to_actor3_tx, actor3_actor1_to_actor3_rx) = base_channel_builder
        .with_capacity(8)
        .build();
    
    let (actor2_actor2_to_actor3_tx, actor3_actor2_to_actor3_rx) = base_channel_builder
        .with_capacity(8)
        .build();
    
    let (actor3_actor3_to_actor4_tx, actor4_actor3_to_actor4_rx) = base_channel_builder
        .with_capacity(8)
        .build();
    
    let (actor4_actor4_to_actor5_tx, actor5_actor4_to_actor5_rx) = base_channel_builder
        .with_capacity(8)
        .build();
    
    //build actors
    
    {
      
    
       base_actor_builder.with_name("Actor1")
                 .build_spawn( move |context| actor::mod_actor1::run(context
                                            , actor1_actor1_to_actor3_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("Actor2")
                 .build_spawn( move |context| actor::mod_actor2::run(context
                                            , actor2_actor2_to_actor3_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("Actor3")
                 .build_spawn( move |context| actor::mod_actor3::run(context
                                            , actor3_actor1_to_actor3_rx.clone()
                                            , actor3_actor2_to_actor3_rx.clone()
                                            , actor3_actor3_to_actor4_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("Actor4")
                 .build_spawn( move |context| actor::mod_actor4::run(context
                                            , actor4_actor3_to_actor4_rx.clone()
                                            , actor4_actor4_to_actor5_tx.clone())
                 );
    }
    {
       base_actor_builder.with_name("Actor5")
                 .build_spawn( move |context| actor::mod_actor5::run(context
                                            , actor5_actor4_to_actor5_rx.clone())
                 );
    }
    graph
}

#[cfg(test)]
mod graph_tests {
    use async_std::test;
    use steady_state::*;

    #[test]
    async fn test_graph_one() {

            let test_ops = Args {
                loglevel: "debug".to_string(),
                systemd_install: false,
                systemd_uninstall: false,
            };
            let mut graph = build_graph(steady_state::Graph::new_test(test_ops.clone()) );
            graph.start();
            let mut guard = graph.sidechannel_director().await;
            if let Some(plane) = guard.deref_mut() {

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