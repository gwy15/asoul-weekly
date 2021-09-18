# build
FROM rust:slim-buster as builder
WORKDIR /code
COPY . .
ENV SQLX_OFFLINE=1
RUN apt update \
    && apt-get install -y clang libclang-dev lld libopencv-dev
RUN cargo b --release --no-default-features --features rustls --bin asoul_weekly \
    && strip target/release/asoul_weekly

# ENTRYPOINT [ "bash" ]
# CMD []

# 
FROM debian:buster-slim
WORKDIR /code
RUN apt update \
    && apt-get install -y libopencv-core-dev libopencv-imgproc-dev libopencv-imgcodecs-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /code/target/release/asoul_weekly .
COPY --from=builder /code/log4rs.yml .

ENTRYPOINT [ "./asoul_weekly" ]
CMD []
