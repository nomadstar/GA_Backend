FROM rustlang/rust:nightly

# Dependencias del sistema
RUN apt-get update && apt-get install -y \
    build-essential libssl-dev pkg-config zlib1g-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app/quickshift

# Copia TODO el proyecto antes de ejecutar cargo
COPY quickshift ./quickshift

# Opcional: precarga dependencias
RUN cargo +nightly fetch

ENV CARGO_BUILD_JOBS=2

EXPOSE 8080

CMD ["cargo", "+nightly", "run"]