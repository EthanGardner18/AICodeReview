mod args;
use structopt::StructOpt;
#[allow(unused_imports)]
use log::*;
use crate::args::Args;
use std::time::Duration;
use steady_state::*;





mod actor {
    
        pub mod api_response_saver;
    
        pub mod api_submitter;
    
        pub mod file_reader;
    
        pub mod file_saver;
    
        pub mod line_number_adder;
    
}

fn main() {
    let opt = Args::from_args();
    if let Err(e) = steady_state::init_logging(&opt.loglevel) {
        //do not use logger to report logger could not start
        eprint!("Warning: Logger initialization failed with {:?}. There will be no logging.", e);
    }

    let service_executable_name = "actor_loop";
    let service_user = "actor_loop_user";
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
    
    let (apiresponsesaver_check_for_files_tx, filereader_check_for_files_rx) = base_channel_builder
        .with_capacity(1)
        .build();
    
    let (apisubmitter_api_response_tx, apiresponsesaver_api_response_rx) = base_channel_builder
        .with_capacity(5)
        .build();
    
    let (filereader_file_content_tx, linenumberadder_file_content_rx) = base_channel_builder
        .with_capacity(5)
        .build();
    
    let (filesaver_saved_file_tx, apisubmitter_saved_file_rx) = base_channel_builder
        .with_capacity(5)
        .build();
    
    let (linenumberadder_numbered_content_tx, filesaver_numbered_content_rx) = base_channel_builder
        .with_capacity(5)
        .build();
    
    //build actors
    
    {
      
    
       base_actor_builder.with_name("ApiResponseSaver")
                 .build_spawn( move |context| actor::api_response_saver::run(context
                                            , apiresponsesaver_api_response_rx.clone()
                                            , apiresponsesaver_check_for_files_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("ApiSubmitter")
                 .build_spawn( move |context| actor::api_submitter::run(context
                                            , apisubmitter_saved_file_rx.clone()
                                            , apisubmitter_api_response_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("FileReader")
                 .build_spawn( move |context| actor::file_reader::run(context
                                            , filereader_check_for_files_rx.clone()
                                            , filereader_file_content_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("FileSaver")
                 .build_spawn( move |context| actor::file_saver::run(context
                                            , filesaver_numbered_content_rx.clone()
                                            , filesaver_saved_file_tx.clone())
                 );
    }
    {
      
    
       base_actor_builder.with_name("LineNumberAdder")
                 .build_spawn( move |context| actor::line_number_adder::run(context
                                            , linenumberadder_file_content_rx.clone()
                                            , linenumberadder_numbered_content_tx.clone())
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