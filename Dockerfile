# ---------- Build stage ----------
FROM rust:1.85-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libfontconfig1-dev libfreetype6-dev libxcb-render0-dev \
    libxcb-shape0-dev libxcb-xfixes0-dev libxcb-xkb-dev libxkbcommon-dev \
    libxkbcommon-x11-dev libwayland-dev libxkbregistry-dev libgtk-3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copia apenas o Cargo primeiro para cachear dependências
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copia o código fonte e builda de verdade
COPY src ./src
COPY config.toml ./
RUN touch src/main.rs && cargo build --release

# ---------- Runtime stage ----------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libfontconfig1 libfreetype6 libxcb-render0 libxcb-shape0 \
    libxcb-xfixes0 libxcb-xkb0 libxkbcommon0 libxkbcommon-x11-0 \
    libgtk-3-0 libwayland-client0 libwayland-cursor0 libwayland-egl1 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/jobhunter /app/jobhunter
COPY config.toml /app/config.toml

# Cria diretório para o banco de dados (volume ou caminho persistente)
RUN mkdir -p /data && chmod 777 /data

ENTRYPOINT ["/app/jobhunter"]
CMD ["gui"]
