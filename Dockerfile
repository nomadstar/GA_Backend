FROM rustlang/rust:nightly

RUN apt-get update && apt-get install -y \
    build-essential libssl-dev pkg-config zlib1g-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app/quickshift

COPY quickshift/Cargo.toml quickshift/Cargo.lock ./
RUN cargo +nightly fetch

COPY quickshift ./quickshift

ENV CARGO_BUILD_JOBS=2

EXPOSE 8080

CMD ["cargo", "+nightly", "run"]