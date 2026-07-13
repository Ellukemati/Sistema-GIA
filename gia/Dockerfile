# ==========================================
# ETAPA 1: Compilación (Builder)
# ==========================================
FROM rust:slim-bullseye AS builder

WORKDIR /app

# 1. Instalamos las dependencias base y wget
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential wget

# 2. Descargamos e instalamos el paquete oficial de wkhtmltopdf (que incluye libwkhtmltox-dev)
RUN wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-2/wkhtmltox_0.12.6.1-2.bullseye_amd64.deb \
    && apt-get install -y ./wkhtmltox_0.12.6.1-2.bullseye_amd64.deb \
    && rm wkhtmltox_0.12.6.1-2.bullseye_amd64.deb

# Le decimos al linker de Rust dónde encontrar la librería para compilar
ENV LIBRARY_PATH=/usr/local/lib:$LIBRARY_PATH
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Copiamos la configuración y el código
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static

# Compilamos
RUN cargo build --release

# ==========================================
# ETAPA 2: Producción (Runtime)
# ==========================================
FROM debian:bullseye-slim

WORKDIR /app

# 3. Para la imagen final instalamos el mismo paquete para tener las librerías dinámicas
# y luego borramos el archivo .deb y la caché para mantener la imagen liviana.
RUN apt-get update && apt-get install -y wget ca-certificates \
    && wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-2/wkhtmltox_0.12.6.1-2.bullseye_amd64.deb \
    && apt-get install -y ./wkhtmltox_0.12.6.1-2.bullseye_amd64.deb \
    && rm wkhtmltox_0.12.6.1-2.bullseye_amd64.deb \
    && rm -rf /var/lib/apt/lists/*

# Le decimos al binario de Rust dónde encontrar la librería al ejecutarse
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Traemos el binario y las carpetas estáticas de la Etapa 1
COPY --from=builder /app/target/release/gia /app/gia
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/static /app/static

RUN mkdir -p /app/data

EXPOSE 8080

CMD ["/app/gia"]