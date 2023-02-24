FROM rust:1.64-slim-buster AS builder

RUN apt-get update -y
RUN apt-get install -y pkg-config openssl libssl-dev

COPY ./src ./src
COPY ./Cargo.* ./

RUN cargo build --release

FROM debian:buster-slim

WORKDIR /app

RUN apt-get update -y
RUN apt-get install -y openssl

COPY --from=builder ./target/release/vrf-server ./vrf-server

COPY ./vrf-server.toml ./

CMD [ "./vrf-server" ]
