# AI Codebase Review Assistant

This is an **AI Codebase Review** project built in **Rust**, utilizing an **async runtime called Steady State**. The goal of this project is to analyze codebases, identify inefficiencies, and provide intelligent suggestions for improvements.

## Features
- AI-powered code analysis
- Asynchronous processing using Steady State
- Efficient detection of inefficient code patterns
- User-friendly suggestions for optimization

## Testing / Development

To test / develop the project download using the installation process below.

For testing purposes you do not have to use the --release option but instead do...

```sh
cargo build
```

this will just compile the debug executable and not the release.

### Installation
```sh
# Clone the repository
git clone https://github.com/AiCodeReview.git
cd AiCodeReview

# Build the project
cargo build --release
```

## Usage
```sh
cargo run --release
```

## The Asynchronous Structure
To include an image in this README, use the following syntax:

![Project-Diagram](images/ai-codebase-review.png)


Replace `assets/logo.png` with the correct path to your image file.

## License
This project is licensed under the MIT License.

