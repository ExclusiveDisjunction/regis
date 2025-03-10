FROM rust:latest

WORKDIR /usr/src/regisd

COPY . /usr/src/regisd
COPY ../

RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev 

RUN cargo build