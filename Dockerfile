# Usa una imagen oficial de Rust como base
FROM rust:1.72-slim

# Instala dependencias del sistema requeridas para construir tu proyecto (ajústalo si es necesario)
RUN apt-get update && apt-get install -y \n    build-essential libssl-dev pkg-config \n    && rm -rf /var/lib/apt/lists/*

# Configura el directorio de trabajo dentro del contenedor
WORKDIR /app

# Copia el archivo de configuración de Rust y dependencias al contenedor
COPY Cargo.toml Cargo.lock ./

# Descarga las dependencias de Rust sin construir el proyecto (mejora el cacheo)
RUN cargo fetch

# Copia el resto del código fuente al contenedor
COPY . .

# Construye tu aplicación en modo release
RUN cargo build --release

# Expone el puerto que usa tu aplicación (ajusta al puerto del servidor en tu código)
EXPOSE 8080

# Comando de inicio del contenedor
CMD ["./target/release/<nombre-de-tu-binario>"]