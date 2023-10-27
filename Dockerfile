FROM rust:1.67-slim-buster AS builder

RUN apt-get update -y
RUN apt-get install -y pkg-config openssl libssl-dev

WORKDIR /builder

COPY ./vrf-sdk /builder/vrf-sdk
COPY ./vrf-server /builder/vrf-server
RUN echo 'workspace={members=["./vrf-sdk","./vrf-server"]}' > Cargo.toml

RUN cargo build --release

FROM debian:buster-slim

WORKDIR /app

RUN apt-get update -y
RUN apt-get install -y openssl

COPY --from=builder /builder/target/release/vrf-server ./vrf-server

COPY ./vrf-server/vrf-server.toml ./

CMD [ "./vrf-server" ]
