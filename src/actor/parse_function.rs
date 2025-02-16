#[allow(unused_imports)]
use log::*;
#[allow(unused_imports)]
use std::time::Duration;
use steady_state::*;
use crate::Args;
use std::error::Error;
use crate::actor::read_file::FileContent;
use serde_json::Value as JsonValue;
use std::fs::OpenOptions;
use std::io::Write;
use serde_json::json;

//if no internal state is required (recommended) feel free to remove this.
#[derive(Default)]
pub(crate) struct ParsefunctionInternalState {
}

#[cfg(not(test))]
pub async fn run(context: SteadyContext
        ,file_content_rx: SteadyRx<FileContent>, state: SteadyState<ParsefunctionInternalState>
    ) -> Result<(),Box<dyn Error>> {

  // if needed CLI Args can be pulled into state from _cli_args
  let _cli_args = context.args::<Args>();
  // monitor consumes context and ensures all the traffic on the chosen channels is monitored
  // monitor and context both implement SteadyCommander. SteadyContext is used to avoid monitoring
  let cmd =  into_monitor!(context, [file_content_rx],[]);
  internal_behavior(cmd,file_content_rx, state).await
}

async fn call_chatgpt_api(file_content: &str, file_path: &str) -> Result<JsonValue, Box<dyn Error>> {
    dotenv::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key not found in environment variables");

    let api_url = "https://api.openai.com/v1/chat/completions";

    let prompt_template = r#"
        You will receive a file of any coding language, the first line will have the path to the file you are looking at. I would like you to parse the code and only store a header for each function in this format. One
        issue you need to check for is that there are comments in the code, so you need to make sure you are starting at the correct line number and ending at the correct line number. Don't forget that different coding
        languages use different methods to comment things in and out. Also if you see a new line assume it counts toward the total line number count. Finally, if the function is within a class, give the class_name:function name. 

        For a function within a class:
        {class_name:function_name, path, starting_line_number, last_line_number}

        For a function without a class:
        {function_name, path, starting_line_number, last_line_number}

        In your response, you should not have anything besides the JSON format structure.
    "#;

    let client = surf::Client::new();
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            { "role": "system", "content": "You are a code parser specializing in analyzing functions." },
            { "role": "user", "content": format!(
                "{}\n\n{}\n\nFile Path: {}\n\nFile Content:\n{}",
                prompt_template.trim(),
                "Parse this file according to the format above.",
                file_path,
                file_content
            )}
        ],
        "max_tokens": 1000,
        "temperature": 0.0
    });

    let mut response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .body(surf::Body::from_json(&request_body)?)
        .await?;

    if response.status().is_success() {
        let response_body: JsonValue = response.body_json().await?;
        Ok(response_body)
    } else {
        let error_message = response.body_string().await?;
        Err(format!("API request failed: {}", error_message).into())
    }
}





fn append_to_file(file_path: &str, content: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    for line in content.lines() {
        let cleaned_line = line.trim() // Removes leading and trailing spaces
            .trim_end_matches(','); // Optional: Removes trailing comma if you don't want it

        if !cleaned_line.is_empty() {
            writeln!(file, "{}", cleaned_line)?;
        }
    }

    Ok(())
}



async fn internal_behavior<C: SteadyCommander>(mut cmd: C,file_content_rx: SteadyRx<FileContent>, state: SteadyState<ParsefunctionInternalState>
 ) -> Result<(),Box<dyn Error>> {

    let mut state_guard = steady_state(&state, || ParsefunctionInternalState::default()).await;
    if let Some(mut state) = state_guard.as_mut() {

   //every read and write channel must be locked for this instance use, this is outside before the loop
   let mut file_content_rx = file_content_rx.lock().await;

   //this is the main loop of the actor, will run until shutdown is requested.
   //the closure is called upon shutdown to determine if we need to postpone the shutdown
   while cmd.is_running(&mut ||file_content_rx.is_closed_and_empty()) {

     // our loop avoids spinning by using await here on multiple criteria. clean is false if await
     // returned early due to a shutdown request or closed channel.
         let clean = await_for_all!(cmd.wait_closed_or_avail_units(&mut file_content_rx,1)    );

  
          //TODO:  here is an example reading from file_content_rx
          match cmd.try_take(&mut file_content_rx) {
              Some(rec) => {
                  trace!("got rec: {:?}", rec);
                   println!("Content Received: {}", rec.content);
                 //  let response = call_chatgpt_api(&rec.content, &rec.path).await;
               // println!("Raw ChatGPT Response: {:?}", response);

            //   match response {
            //         Ok(json) => {
            //             if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
            //                 let content_cleaned = content
            //                     .strip_prefix("```json\n")
            //                     .and_then(|s| s.strip_suffix("```"))
            //                     .unwrap_or(content)
            //                     .trim();

            //                 // Extract only what's inside the brackets []
            //                 let content_inside_brackets = content_cleaned
            //                     .strip_prefix('[')
            //                     .and_then(|s| s.strip_suffix(']'))
            //                     .unwrap_or(content_cleaned)
            //                     .trim();

            //                 println!("Extracted ChatGPT API response:\n{}", content_inside_brackets);

            //                 if let Err(e) = append_to_file("test.txt", content_inside_brackets) {
            //                     eprintln!("Failed to write to file: {}", e);
            //                 }
            //             } else {
            //                 eprintln!("Unexpected JSON structure: {}", json);
            //             }
            //         }
            //         Err(e) => {
            //             eprintln!("Failed to call ChatGPT API: {}", e);
            //         }
            //     }

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
        ,file_content_rx: SteadyRx<FileContent>, state: SteadyState<ParsefunctionInternalState>
    ) -> Result<(),Box<dyn Error>> {
    let mut cmd =  into_monitor!(context, [file_content_rx],[]);
    if let Some(responder) = cmd.sidechannel_responder() {
         let mut file_content_rx = file_content_rx.lock().await;
         while cmd.is_running(&mut ||
             file_content_rx.is_closed_and_empty()) {
                // in main use graph.sidechannel_director node_call(msg,"ParseFunction")
                let _did_check = responder.equals_responder(&mut cmd,&mut file_content_rx).await;
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
       let (test_file_content_tx,file_content_rx) = graph.channel_builder().with_capacity(4).build();
       let state = new_state();
       graph.actor_builder()
                    .with_name("UnitTest")
                    .build_spawn( move |context|
                            internal_behavior(context, file_content_rx.clone(), state.clone())
                     );

       graph.start(); //startup the graph
       //TODO:  adjust this vec content to make a valid test
       test_file_content_tx.testing_send_all(vec![FileContent::default()],true).await;

        
       graph.request_stop();
       graph.block_until_stopped(Duration::from_secs(15));}
}
