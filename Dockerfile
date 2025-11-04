# Especificar una imagen base oficial para Rust
FROM rust:1.72-slim

# Configurar el directorio de trabajo dentro del contenedor
WORKDIR /app/quickshift

# Copiar los archivos necesarios y configurar dependencias
COPY quickshift/Cargo.toml quickshift/Cargo.lock ./

# Descarga y guarda las dependencias con `cargo` en caché
RUN cargo fetch

# Copiar el resto del código fuente
COPY quickshift/ .

# Construir el ejecutable en modo release
RUN cargo build --release

# Exponer el puerto de escucha para ejecutar el servidor
EXPOSE 8080

# Comando para iniciar la aplicación
CMD ["cargo", "run", "--release"]