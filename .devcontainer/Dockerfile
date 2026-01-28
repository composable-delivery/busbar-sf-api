# Dockerfile
FROM rust:latest

# Set the working directory
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Install dependencies
RUN cargo build --release

# Copy the source code
COPY ./src ./src

# Set the entrypoint
CMD ["cargo", "run"]
