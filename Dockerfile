# Use an official Rust image
FROM rust:1.72-slim

# Install system dependencies (adjust as required)
RUN apt-get update && apt-get install -y \
    build-essential libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory to quickshift module inside the container
WORKDIR /app/quickshift

# Copy the application and its dependencies from the host into the image
COPY quickshift quickshift
COPY Cargo.toml Cargo.lock ../

# Fetch dependencies without building
RUN cargo fetch

# Expose the application port
EXPOSE 8080

# Run the application
CMD ["cargo", "run"]
