# Build image
FROM rust:slim-bullseye@sha256:12d9a0ff4f3c87badbf56a27ffd6c4774ebe1b5fe5c6b7b1a39cfee537fcb62f as build

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
FROM debian:bullseye-slim@sha256:52927eff8153b563244f98cdc802ba97918afcdf67f9e4867cbf1f7afb3d147b

RUN apt-get update && apt-get install -y python3-pip
RUN pip install -U yt-dlp

COPY --from=build /app/target/release/ohrwurm .

CMD ["./ohrwurm"]
