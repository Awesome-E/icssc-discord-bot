FROM rust:1.81
COPY ./ ./
RUN cargo build --release
CMD ["./target/release/matchy_meetups_bot"]
