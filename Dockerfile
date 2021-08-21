# build
FROM rust:slim-buster as builder
WORKDIR /code
COPY . .
RUN cargo b --release --no-default-features --features rustls && strip target/release/asoul_weekly

# 
FROM debian:buster-slim
WORKDIR /code
COPY --from=builder /code/target/release/asoul_weekly .
COPY --from=builder /code/log4rs.yml .
ENTRYPOINT [ "./asoul_weekly" ]
CMD []
