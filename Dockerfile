# Especificar una imagen base oficial para Rust
# Usar la etiqueta 'latest-slim' para asegurar una toolchain moderna que soporte la
# edición 2024 declarada en `quickshift/Cargo.toml`. Si prefieres reproducibilidad,
# podemos fijar una versión concreta compatible (p. ej. 'rust:1.78-slim') en vez de
# 'latest-slim'.
FROM rust:1.91-slim

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
# Instalar dependencias del sistema necesarias para crates que enlazan con
# libfontconfig / freetype (plotters, fontconfig bindings, etc.).
# `pkg-config` es requerido por varios build-scripts (yeslogic-fontconfig-sys).
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		build-essential \
		pkg-config \
		libfontconfig1-dev \
		libfreetype6-dev \
		ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

# Compilar en release (puede tardar varios minutos dentro del contenedor)
RUN cargo build --release

# Exponer el puerto de escucha para ejecutar el servidor
EXPOSE 8080

# Comando para iniciar la aplicación
CMD ["cargo", "run", "--release"]