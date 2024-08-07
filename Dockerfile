FROM rust:bullseye as builder

RUN apt update && \
    apt install -y libdbus-1-dev

WORKDIR /home

COPY . .

RUN cargo build -j $(nproc) --release


FROM scratch as artifacts

COPY --from=builder /home/target/release /
