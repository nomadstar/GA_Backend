# Directorio de trabajo
WORKDIR /app/quickshift

# Copia solo los archivos de dependencias primero
COPY quickshift/Cargo.toml quickshift/Cargo.lock ./

# Precompila todas las dependencias (incluyendo Polars)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo +nightly build --release
RUN rm -rf src  # eliminamos el main dummy

# Copia todo el contenido del proyecto
COPY quickshift/ ./   # <- copia el contenido dentro del WORKDIR correctamente

# Limita jobs de compilación (opcional)
ENV CARGO_BUILD_JOBS=2

# Exponer puerto
EXPOSE 8080

# Ejecutar el proyecto
CMD ["cargo", "+nightly", "run"]