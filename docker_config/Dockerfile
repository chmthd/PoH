# Use an official Rust image as a base
FROM rust:latest

RUN apt-get update && apt-get install -y netcat-openbsd
# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the contents of the POHTS directory into the container
COPY . .

# Build the application
RUN cargo build --release

# Expose the ports needed for the network nodes (just for documentation)
# Docker Compose is responsible for mapping ports to the host machine.
EXPOSE 8080  
EXPOSE 8081  
EXPOSE 8082  

# Command to run the application (this will be overridden by docker-compose.yaml)
CMD ["cargo", "run", "--release"]