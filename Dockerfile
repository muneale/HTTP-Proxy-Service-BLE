FROM rust:bullseye

RUN apt update && \
    apt install -y libdbus-1-dev

WORKDIR /home

COPY . .

RUN cargo -j $(nproc) build --release
