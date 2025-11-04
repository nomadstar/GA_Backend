# Usa Rust nightly oficial
FROM rustlang/rust:nightly

# Instala dependencias del sistema necesarias
RUN apt-get update && apt-get install -y \
    build-essential libssl-dev pkg-config zlib1g-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

# Configura el directorio de trabajo
WORKDIR quickshift

# Copia solo los archivos de dependencias primero
COPY quickshift/Cargo.toml quickshift/Cargo.lock ./

# Precompila todas las dependencias (incluyendo Polars)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo +nightly build --release
RUN rm -rf src  # eliminamos el main dummy

# Copia el resto del código
COPY quickshift ./quickshift/*

# Limita jobs de compilación (opcional)
ENV CARGO_BUILD_JOBS=2

# Exponer puerto
EXPOSE 8080

# Ejecutar el proyecto
CMD ["cargo", "+nightly", "run"]