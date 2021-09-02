# build
FROM rust:slim-buster as builder
WORKDIR /code
COPY . .
ENV SQLX_OFFLINE=1
RUN RUN apt update \
    && apt-get install -y clang lld \
    && cargo b --release --no-default-features --features rustls --bin asoul_weekly \
    && strip target/release/asoul_weekly

# 
FROM debian:buster-slim
WORKDIR /code
COPY --from=builder /code/target/release/asoul_weekly .
COPY --from=builder /code/log4rs.yml .
ENTRYPOINT [ "./asoul_weekly" ]
CMD []
