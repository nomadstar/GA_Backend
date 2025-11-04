# Especificar una imagen base oficial para Rust
FROM rust:1.72-slim

# Configurar el directorio de trabajo dentro del contenedor

WORKDIR /app/quickshift

# Copiar todo el crate `quickshift` al contexto del contenedor.
# Copiar todo evita errores de manifest (faltan targets) durante las etapas de build
# y es más simple; si quieres mantener cache de dependencias podríamos copiar
# Cargo.toml y src/ por separado, pero aquí priorizamos fiabilidad.
COPY quickshift/ .

# Descargar dependencias (cargo fetch) puede hacerse ahora si se desea aprovechar cache,
# pero `cargo build` también las descargará. Mantengo un cargo fetch explícito para
# acelerar builds incrementales cuando se reusa la capa de Docker.
RUN cargo fetch

# Construir el ejecutable en modo release
RUN cargo build --release

# Exponer el puerto de escucha para ejecutar el servidor
EXPOSE 8080

# Comando para iniciar la aplicación
CMD ["cargo", "run", "--release"]