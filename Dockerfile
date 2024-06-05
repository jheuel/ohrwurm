# Build image
FROM rust:slim-bullseye as build

RUN apt-get update && apt-get install -y \
    build-essential autoconf automake cmake libtool libssl-dev pkg-config

WORKDIR "/app"

# Cache dependencies
COPY Cargo.toml .
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --locked

# Build
COPY . .
RUN cargo build --release --locked

# Release image
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y python3-pip ffmpeg
RUN pip install -U yt-dlp

COPY --from=build /app/target/release/ohrwurm .

CMD ["./ohrwurm"]
