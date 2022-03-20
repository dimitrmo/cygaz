FROM rust:1.59.0-bullseye as builder
WORKDIR /usr/src/cygaz
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && \
    apt-get install -y \
        curl \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates
COPY --from=builder /usr/local/cargo/bin/cygaz /usr/local/bin/cygaz

LABEL org.opencontainers.image.description="Cyprus Gas Prices"

ENV TIMEOUT     600000
ENV HOST        0.0.0.0
ENV PORT        8080
ENV RUST_LOG    cygaz=info

EXPOSE          8080

CMD ["cygaz"]
