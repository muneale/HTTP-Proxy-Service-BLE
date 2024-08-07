FROM rust:bullseye AS builder

RUN apt update && \
    apt install -y libdbus-1-dev

WORKDIR /home

COPY . .

RUN cargo build -j $(nproc) --release


FROM scratch AS artifacts

COPY --from=builder /home/target/release /
