FROM rustlang/rust:nightly

# Dependencias del sistema
RUN apt-get update && apt-get install -y \
    build-essential libssl-dev pkg-config zlib1g-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

# Directorio de trabajo
WORKDIR /app/quickshift

# Copia todo el contenido del proyecto
COPY quickshift/ ./

# Opcional: precarga dependencias
RUN cargo +nightly fetch

# Limita los jobs de compilación (opcional)
ENV CARGO_BUILD_JOBS=2

# Exponer puerto
EXPOSE 8080

# Ejecutar el proyecto
CMD ["cargo", "+nightly", "run"]