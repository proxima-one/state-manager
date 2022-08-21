FROM rust:latest as build

RUN apt-get update && apt-get install -y libclang-dev cmake

RUN USER=root cargo new --bin app
WORKDIR /app

COPY ./Cargo.* ./
RUN cargo build --release && rm src/*.rs ./target/release/deps/state_manager*

COPY ./build.rs ./
COPY ./proto ./proto/
COPY ./src ./src/
RUN cargo build --release


FROM rust:slim as prod

USER root

COPY --from=build /app/target/release/state-manager /usr/local/bin/state-manager
CMD ["/usr/local/bin/state-manager"]
