mod args;
use structopt::StructOpt;
#[allow(unused_imports)]
use log::*;
use crate::args::Args;
use std::time::Duration;
use steady_state::*;

mod actor {
    
        pub mod archive;
        pub mod function_reviewer;
        pub mod function_scraper;
        pub mod function_storer;
        pub mod parse_function;
        pub mod read_directory;
        pub mod read_file;
}

fn main() {
    let opt = Args::from_args();
    if let Err(e) = init_logging(&opt.loglevel) {
        //do not use logger to report logger could not start
        eprint!("Warning: Logger initialization failed with {:?}. There will be no logging.", e);
    }

    let service_executable_name = "ai-codebase-review-ld";
    let service_user = "ai-codebase-review-ld_user";
    let systemd_command = SystemdBuilder::process_systemd_commands(  opt.systemd_action()
                                                   , opt.to_cli_string(service_executable_name)
                                                   , service_executable_name
                                                   , service_user);

    if !systemd_command {
        info!("Starting up");
        let mut graph = build_graph(GraphBuilder::default().build(opt.clone()) );
        graph.start();

        {  //remove this block to run forever.
           std::thread::sleep(Duration::from_secs(60));
           graph.request_stop(); //actors can also call stop as desired on the context or monitor
        }

        graph.block_until_stopped(Duration::from_secs(2));
    }
}

fn build_graph(mut graph: Graph) -> Graph {

    //this common root of the channel builder allows for common config of all channels
    let base_channel_builder = graph.channel_builder()
        .with_type()
        .with_line_expansion(1.0f32);

    //this common root of the actor builder allows for common config of all actors
    let base_actor_builder = graph.actor_builder()
        .with_mcpu_percentile(Percentile::p80())
        .with_load_percentile(Percentile::p80());

    //build channels
    
    let (archive_archived_function_tx, archive_to_function_scraper_archived_function_rx) = base_channel_builder
        .with_capacity(50)
        .build_as_bundle::<_,2>();
    
    let (functionreviewer_reviewed_function_tx, archive_reviewed_function_rx) = base_channel_builder
        .with_capacity(50)
        .build();
    
    let (functionscraper_scraped_function_tx, functionreviewer_scraped_function_rx) = base_channel_builder
        .with_capacity(50)
        .build();
    
    let (parsefunction_parsed_function_tx, functionscraper_parsed_function_rx) = base_channel_builder
        .with_capacity(50)
        .build();
    
    let (readdirectory_directory_tx, readfile_directory_rx) = base_channel_builder
        .with_capacity(10)
        .build();
    
    let (readfile_file_tx, parsefunction_file_rx) = base_channel_builder
        .with_capacity(20)
        .build();
    
    //build actors
    
    {
     let state = new_state();
    
     base_actor_builder.with_name("Archive")
                 .build( move |context| actor::archive::run(context
                                            , archive_reviewed_function_rx.clone()
                                            , archive_archived_function_tx.clone(), state.clone() )
                  , &mut Threading::Spawn );
    }
    {
     let state = new_state();
    
     base_actor_builder.with_name("FunctionReviewer")
                 .build( move |context| actor::function_reviewer::run(context
                                            , functionreviewer_scraped_function_rx.clone()
                                            , functionreviewer_reviewed_function_tx.clone(), state.clone() )
                  , &mut Threading::Spawn );
    }
    {
     let state = new_state();
    let functionscraper_archived_function_rx = archive_to_function_scraper_archived_function_rx[0].clone();
     base_actor_builder.with_name("FunctionScraper")
                 .build( move |context| actor::function_scraper::run(context
                                            , functionscraper_archived_function_rx.clone()
                                            , functionscraper_parsed_function_rx.clone()
                                            , functionscraper_scraped_function_tx.clone(), state.clone() )
                  , &mut Threading::Spawn );
    }
    // !{
    //  !let state = new_state();
    // !let functionstorer_archived_function_rx = archive_to_function_storer_archived_function_rx[0].clone();
    //  base_actor_builder.with_name("FunctionStorer")
    //              .build( move |context| actor::function_storer::run(context
    //                                         , functionstorer_archived_function_rx.clone(), state.clone() )
    //               , &mut Threading::Spawn );
    // }
    {
     let state = new_state();
    
     base_actor_builder.with_name("ParseFunction")
                 .build( move |context| actor::parse_function::run(context
                                            , parsefunction_file_rx.clone()
                                            , parsefunction_parsed_function_tx.clone(), state.clone() )
                  , &mut Threading::Spawn );
    }
    {
     let state = new_state();
    
     base_actor_builder.with_name("ReadDirectory")
                 .build( move |context| actor::read_directory::run(context
                                            , readdirectory_directory_tx.clone(), state.clone() )
                  , &mut Threading::Spawn );
    }
    {
     let state = new_state();
    
     base_actor_builder.with_name("ReadFile")
                 .build( move |context| actor::read_file::run(context
                                            , readfile_directory_rx.clone()
                                            , readfile_file_tx.clone(), state.clone() )
                  , &mut Threading::Spawn );
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
    use futures_timer::Delay;

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

             //NOTE: to ensure the node_call is for the correct channel for a given actor unique types for each channel are required

            
                     //TODO:   Adjust as needed to inject test values into the graph
                     //  let response = plane.call_actor(Box::new(ArchivedFunction::default()), "FunctionStorer").await;
                     //  if let Some(msg) = response { // ok indicates the message was echoed
                     //     //trace!("response: {:?} {:?}", msg.downcast_ref::<String>(),i);
                     //     assert_eq!("ok", msg.downcast_ref::<String>().expect("bad type"));
                     //  } else {
                     //     error!("bad response from generator: {:?}", response);
                     //    // panic!("bad response from generator: {:?}", response);
                     //  }
                
                

              // //TODO:   if needed you may want to add a delay right here to allow the graph to process the message
              Delay::new(Duration::from_millis(100)).await;

             
                
                     //TODO:   Adjust as needed to test the values produced by the graph
                     //  let response = plane.call_actor(Box::new(DirectoryPath::default()), "ReadDirectory").await;
                     //  if let Some(msg) = response { // ok indicates the expected structure instance matched
                     //     //trace!("response: {:?} {:?}", msg.downcast_ref::<String>(),i);
                     //     assert_eq!("ok", msg.downcast_ref::<String>().expect("bad type"));
                     //  } else {
                     //     error!("bad response from generator: {:?}", response);
                     //    // panic!("bad response from generator: {:?}", response);
                     //  }
                

            }
            drop(guard);
            graph.request_stop();
            graph.block_until_stopped(Duration::from_secs(3));

    }
}