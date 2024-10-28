
use ureq::Agent;
use serde_json::Value;
use serde_json::json;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

// Function to send the request to OpenAI API
fn sendOpenAIRequest(prompt: &str, filePath: &str) -> Result<String, Box<dyn std::error::Error>> {
    let apiKey = "sk-dKkRWlmUofaExHEFJQRi0QRG-Nq1IxPOk0Zq5C_JvpT3BlbkFJd41IeOSaBXNQjwEyMDrS33MigiW_FW_gvSpIoK3fMA"; 
    let agent = Agent::new();

    // Add a preamble for context and concatenate the file contents
    let fullPrompt = format!("You will recieve a file of any coding language with the path: {}, the first line will have the path to the file you are looking at. I would like you to parse the code and only store a header for each function in this format. One issue you need to check for is that there are comments in the code, so you need to make sure you are starting at the correct line number and ending at the correct line number. Don't forget that different coding languages use different methods to comment things in and out. Also if you see a new line asssume it counts toward the total line number count. Finally if the function is within a class, give the class name:function name: {{Function Name, Path, Starting Line Number, Last Line Number}} If function is within a class {{dataGen:load_data, /functions/main.py, 6, 14}} {{load_data, /functions/main.py, 6, 14}}. After each parse add a new line.\n{}",filePath, prompt);

    let response = agent
    .post("https://api.openai.com/v1/chat/completions")
    .set("Authorization", &format!("Bearer {}", apiKey))
    .set("Content-Type", "application/json")
    .send_json(json!({
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "Respond with only the final answer, no explanations."},
            {"role": "user", "content": fullPrompt.trim()}
        ],
        "max_tokens": 1000,
        "temperature": 0
    }))?;  // Handle any potential errors from sending

    if (200..300).contains(&response.status()){
        let response_text = response.into_string()?;
    
        // Parse the response text to extract the relevant content
        let json_response: Value = serde_json::from_str(&response_text)?;
        let content = json_response["choices"][0]["message"]["content"].as_str().unwrap();
        
        Ok(content.to_string())
    } else {
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Request failed")))
    }
}

// Function to get the file path input from the user
fn getFilePath() -> String {
    loop {
        println!("Please enter the file path:");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let mut filePath = input.trim().to_string();

        // Trim surrounding quotes if present
        if filePath.starts_with('"') && filePath.ends_with('"') {
            filePath = filePath[1..filePath.len()-1].to_string();
        }

        let path = Path::new(&filePath);

        if path.exists() {
            return filePath; // Return the valid file path
        } else {
            println!("The file does not exist. Please try again.");
        }
    }
}

// Function to read the content from the file
fn readFileContents(filePath: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(filePath);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

// Function to add line numbers to each line of the file
fn addLineNumbers(contents: &str) -> String {
    contents
        .lines()
        .enumerate()
        .map(|(i, line)| format!("{}: {}", i + 1, line))  // Line numbers start from 1
        .collect::<Vec<String>>()
        .join("\n")
}


// Function to write the response to a file
fn writeToFile(contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut outputFile = File::create("codeReview.txt")?;
    outputFile.write_all(contents.as_bytes())?;
    println!("Code review saved to codeReview.txt.");
    Ok(())
}

// Main function
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the file path from the user
    let filePath = getFilePath();

    // Read the contents of the file
    let contents = readFileContents(&filePath)?;

    // Add line numbers to the file content
    let numberedContents = addLineNumbers(&contents);

    // Send the contents to the OpenAI API and get the response
    match sendOpenAIRequest(&numberedContents, &filePath) {
        Ok(response) => writeToFile(&response)?,
        Err(e) => println!("Error: {}", e),
    }
    Ok(())
}
