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
RUN rm /app/target/release/ohrwurm

COPY . .
RUN cargo build --release --locked && cp /app/target/release/ohrwurm /ohrwurm

# Release image
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y python3-pip ffmpeg
RUN pip install -U yt-dlp

COPY --from=build /ohrwurm .

CMD ["./ohrwurm"]
