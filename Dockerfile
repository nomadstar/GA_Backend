# Usa una imagen bas# Usa una imagen basada en Rust nightly
FROM rustlang/rust:nightly

# Instala dependencias del sistema necesarias para construir tu proyecto
RUN apt-get update && apt-get install -y \
    build-essential libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Configura el directorio de trabajo dentro del contenedor
WORKDIR /app/quickshift

# Copia todo el contenido del directorio quickshift
COPY quickshift /app/quickshift

# Copia los archivos Cargo.toml y Cargo.lock
COPY quickshift/Cargo.toml quickshift/Cargo.lock ./

# Solo fetch de dependencias, no reinstales nightly
RUN cargo +nightly fetch

# Exponer los puertos (ajusta según tu proyecto)
EXPOSE 8080

# Ejecutar el servidor utilizando cargo y build
CMD ["cargo", "+nightly", "run"]