# Build image
FROM rust:slim-bullseye@sha256:32dcc2fe5087d439348bd1b63c97232830682009c49bccfe80cf768d8cd40bd7 as build

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
FROM debian:bullseye-slim@sha256:779034981fec838da124ff6ab9211499ba5d4e769dabdfd6c42c6ae2553b9a3b

RUN apt-get update && apt-get install -y python3-pip
RUN pip install -U yt-dlp

COPY --from=build /app/target/release/ohrwurm .

CMD ["./ohrwurm"]
