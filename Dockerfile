FROM rust:slim-trixie AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates/powergrid-core crates/powergrid-core
COPY crates/powergrid-server crates/powergrid-server
# Stub out the client so the workspace builds without its heavy dependencies
RUN mkdir -p crates/powergrid-client/src && echo 'fn main(){}' > crates/powergrid-client/src/main.rs
COPY crates/powergrid-client/Cargo.toml crates/powergrid-client/Cargo.toml

RUN cargo build --release -p powergrid-server

FROM debian:trixie-slim

WORKDIR /app
COPY --from=builder /app/target/release/powergrid-server ./
COPY maps/ maps/

ENV PORT=3000
ENV MAP_FILE=maps/germany.toml

EXPOSE 3000

CMD ["./powergrid-server"]
