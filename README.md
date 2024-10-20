# AI Response System

This Rust program is designed to interact with OpenAI's API, handle user inputs, and manage communication between different components using channels. The system asynchronously sends user input to OpenAI's API, receives a response, and stores it in a file. The program uses the `steady_state` actor-based framework and handles message passing between different actors.

## How It Works

1. **User Input Receiver**: Accepts user input and sends it to OpenAI's API asynchronously.
2. **OpenAI API Caller**: Calls the OpenAI API with the user input, retrieves the response, and sends it to another actor.
3. **Response Printer**: Receives the response from the OpenAI API and prints it to the console while saving the response to a file.

## Prerequisites

- Rust installed on your machine. [Install Rust](https://www.rust-lang.org/tools/install) if you haven't already.
- OpenAI API key. You can get it by signing up at [OpenAI](https://platform.openai.com/api-keys).

## Project Structure

```bash
src/
│
├── main.rs                    # Entry point of the program
├── args.rs                    # Handles command-line arguments
└── actor/                     # Contains actor modules
    ├── input_receiver.rs      # Handles user input
    ├── aisender.rs            # Sends user input to OpenAI API
    └── response_printer.rs    # Receives AI response and prints it
```
## Installation

Clone the repository and navigate to the project directory:

```bash
git clone https://github.com/EthanGardner18/AICodeReview/tree/llm_actors.git
cd llm_actors
```

To compile the code do:

```bash
cargo build
```

## How to Run

To run the program do:

```bash
cargo run
```

This will start the program, prompt for user input, and interact with OpenAI's API based on your input.

## Getting Docs for the Program

The cargo ```doc --open command``` is used to generate and open the documentation for your Rust project in a web browser. It generates the documentation by analyzing your code comments and opens a web page with a structured and clickable format.

To generate and view the documentation:
```bash
cargo doc --open
```
This will create an HTML file with all the project documentation, including the function descriptions, modules, and structs. It is a great way to explore the codebase through generated docs.

## API Key Setup

Save your api key in the ai_sender.rs like this:

```rust
let api_key = "your-api-key-here";
```
THIS IS NOT A SECURE WAY TO HANDLE API KEY SO BE WEARY.