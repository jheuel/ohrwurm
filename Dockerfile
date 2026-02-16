# Build image
FROM rust:slim-bullseye@sha256:eca54108db942b7003a5753cae5588004d0f06e9df8a9bad9e28af17dbd8a8ea as build

RUN apt-get update && apt-get install -y \
    build-essential autoconf automake cmake libtool libssl-dev pkg-config

WORKDIR "/app"

# Cache dependencies
COPY Cargo.toml Cargo.lock .
RUN mkdir src \
    && echo 'fn main() { panic!("Dummy function called!"); }' > ./src/main.rs
RUN cargo build --release --locked

# Build
COPY . .
RUN touch src/main.rs
RUN cargo build --release --locked

# Release image
FROM debian:bullseye-slim@sha256:f0dbd70ae23f6ffa17f8b816a1ba1a489f7e9b3c32328867f6b456dec869e031

RUN apt-get update && apt-get install -y python3-pip
RUN pip install -U yt-dlp

COPY --from=build /app/target/release/ohrwurm .

CMD ["./ohrwurm"]
