# Build image
FROM rust:slim-bullseye@sha256:7b85733174e7e5bb18d04389fe4f44c0a2421001c0fac80e9b25856fa5b077e1 as build

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
FROM debian:bullseye-slim@sha256:2f2307d7c75315ca7561e17a4e3aa95d58837f326954af08514044e8286e6d65

RUN apt-get update && apt-get install -y python3-pip
RUN pip install -U yt-dlp

COPY --from=build /app/target/release/ohrwurm .

CMD ["./ohrwurm"]
