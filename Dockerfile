FROM clux/muslrust:stable as build

RUN apt-get update && apt-get install -y libpq-dev openssl pkg-config clang libssl-dev
RUN rustup target add --toolchain stable x86_64-unknown-linux-gnu
RUN rustup component add rustfmt
ENV PATH="${PATH}:${HOME}/.cargo/bin"

WORKDIR /app
RUN USER=root cargo init --vcs none

# COPY rust-toolchain ./
RUN rustc --version && rustup target add x86_64-unknown-linux-musl

# Static linking for C++ code
RUN ln -s "/usr/bin/g++" "/usr/bin/musl-g++"

COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --locked

RUN apt-get install -y cmake
COPY . ./

# cargo apparently uses mtime and docker doesn't modify it, needed to rebuild:
RUN touch src/main.rs
RUN cargo build --release --locked

FROM alpine:3.16.2 as prod

RUN apk --no-cache add ca-certificates
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/state-manager /usr/local/bin/state-manager
CMD ["/usr/local/bin/state-manager"]
