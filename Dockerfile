# docker build -t ai-codebase-reviewer .

# Start from the official Rust image to ensure we have the latest version of Rust and Cargo
FROM rust:latest as builder

# Set the working directory inside the container
WORKDIR /usr/src/ai-codebase-reviewer

# Copy the actual source code of the Rust project into the Docker image
COPY . .
RUN cargo fetch
# above this point we hope to have cached all our crates

RUN cargo build
RUN cargo test

RUN cargo install
# new layer and copy

# Start a new build stage to create a smaller final image
FROM debian:buster-slim

# Copy the binary from the builder stage to the final image
COPY --from=builder /usr/local/cargo/bin/ai-codebase-reviewer /usr/local/bin/ai-codebase-reviewer

# Set the default command for the container
CMD ["ai-codebase-reviewer"]