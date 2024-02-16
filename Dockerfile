# Build image
FROM rust:slim-bullseye as build

RUN apt-get update && apt-get install -y \
    build-essential autoconf automake cmake libtool libssl-dev pkg-config

WORKDIR "/app"

# Cache cargo build dependencies by creating a dummy source
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
COPY Cargo.toml ./
COPY Cargo.lock ./
RUN cargo build --release --locked

COPY . .
RUN cargo build --release --locked

# Release image
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y python3-pip ffmpeg
RUN pip install -U yt-dlp

COPY --from=build /app/target/release/djheuel .

CMD ["./djheuel"]
