
#[allow(unused_imports)]
use log::*;
use std::default;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;


use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Default,Clone,Debug,Eq,PartialEq)]
pub(crate) struct DirectoryPath {
   path: PathBuf //TODO:  remove dummy and put your channel message fields here
}

// impl Default for DirectoryPath {
//   fn default() -> Self {
//       DirectoryPath {
//                        // Default age
//       }
//   }
// }

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ReaddirectoryInternalState {
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,directory_tx: SteadyTx<DirectoryPath>, state: SteadyState<ReaddirectoryInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [],[directory_tx]);
  internal_behavior(cmd, directory_tx, state).await
}

async fn internal_behavior<C: SteadyCommander>(mut cmd: C,directory_tx: SteadyTx<DirectoryPath>, state: SteadyState<ReaddirectoryInternalState>
 ) -> Result<(),Box<dyn Error>> {

  let user_input = get_user_input();

    let mut state_guard = steady_state(&state, || ReaddirectoryInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut directory_tx = directory_tx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||directory_tx.mark_closed()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         // !let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut start_rx,1)    );
         let mut default_path: PathBuf = PathBuf::new();
         let mut file_path = DirectoryPath {
          path: default_path
         };

         match find_code_files(&user_input) {
          Ok(code_files) => {
              for file in code_files {
                  // ?println!("Found code file: {}", file.display());

                  file_path.path = file;
                  

                //TODO:  here is an example writing to directory_tx
                match cmd.try_send(&mut directory_tx, DirectoryPath::default() ) {
                  Ok(()) => {
                  },
                  Err(msg) => { //in the above await we should have confirmed space is available
                      trace!("error sending: {:?}", msg)
                  },
                } 
              }
          }
          Err(e) => {
              eprintln!("Error: {}", e);
          }
      }

  

  

      }
    }
    Ok(())
}


#[cfg(test)]
pub async fn run(context: SteadyContext
        ,directory_tx: SteadyTx<DirectoryPath>, state: SteadyState<ReaddirectoryInternalState>
    ) -> Result<(),Box<dyn Error>> {
    let mut cmd =  into_monitor!(context, [],[directory_tx]);
    if let Some(responder) = cmd.sidechannel_responder() {
         let mut directory_tx = directory_tx.lock().await;
         while cmd.is_running(&mut ||directory_tx.mark_closed()) {
                 // in main use graph.sidechannel_director node_call(msg,"ReadDirectory")
                 let _did_echo = responder.echo_responder(&mut cmd,&mut directory_tx).await;
         }
    }
    Ok(())
}


/// Traverses a directory and finds all code files within it.
///
/// # Arguments
/// * `dir_path` - A string slice representing the directory path.
///
/// # Returns
/// * A `Result` containing a `Vec<PathBuf>` of paths to the code files or an `io::Error` on failure.
pub fn find_code_files(dir_path: &str) -> io::Result<Vec<PathBuf>> {
  let mut code_files = Vec::new();
  let dir = Path::new(dir_path);

  if !dir.exists() || !dir.is_dir() {
      return Err(io::Error::new(io::ErrorKind::NotFound, "Invalid directory path"));
  }

  traverse_directory(dir, &mut code_files)?;
  Ok(code_files)
}

/// Recursively traverses a directory to find code files.
///
/// # Arguments
/// * `dir` - A reference to the directory path.
/// * `code_files` - A mutable reference to a vector of `PathBuf` to store found code file paths.
fn traverse_directory(dir: &Path, code_files: &mut Vec<PathBuf>) -> io::Result<()> {
  for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();

      if path.is_dir() {
          // Recursively process subdirectories
          traverse_directory(&path, code_files)?;
      } else if path.is_file() {
          if is_code_file(&path) {
              code_files.push(path);
          }
      }
  }
  Ok(())
}

/// Checks if a file is a code file based on its extension.
///
/// # Arguments
/// * `path` - A reference to the file path.
///
/// # Returns
/// * `true` if the file is a code file, otherwise `false`.
fn is_code_file(path: &Path) -> bool {
  let code_extensions = [
      "as", "ascx", "asm", "asp", "aspx", "bas", "c", "c++", "cc", "clj", "coffee", "cpp",
      "cs", "dart", "el", "elm", "erl", "ex", "exs", "f90", "f95", "fs", "fsi", "fsx",
      "go", "groovy", "h", "h++", "hh", "hpp", "hs", "htm", "html", "hxx", "ino", "java",
      "jl", "js", "jsx", "kt", "kts", "lisp", "lua", "m", "ml", "mli", "nim", "php", "pl",
      "pm", "py", "r", "rb", "rs", "sass", "scala", "scm", "scss", "sh", "sml", "sql",
      "styl", "swift", "tex", "ts", "tsx", "vb", "vbs", "vue", "zsh", "cbl",
  ];

  if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
      code_extensions.contains(&extension)
  } else {
      false
  }
}

//gets user input in the terminal
fn get_user_input() -> String {
  let mut input = String::new();

  println!("Enter your input: ");
  io::stdin()
      .read_line(&mut input)
      .expect("Failed to read input");

  input.trim().to_string()
}









#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;
    use steady_state::*;
    use super::*;

    #[async_std::test]
    pub(crate) async fn test_simple_process() {
       let mut graph = GraphBuilder::for_testing().build(());
       let (directory_tx,test_directory_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, directory_tx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));
       //TODO:  confirm values on the output channels
       //    assert_eq!(test_directory_rx.testing_avail_units().await, 1); // check expected count
       let results_directory_vec = test_directory_rx.testing_take().await;
        }
}