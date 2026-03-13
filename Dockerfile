FROM rustlang/rust:nightly AS builder

WORKDIR /usr/src/app
COPY . .

RUN rustup toolchain install nightly-2025-01-20 && \
    rustup default nightly-2025-01-20

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3-pip \
    python3-full \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install yt-dlp --break-system-packages

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/xigma-md .
COPY --from=builder /usr/src/app/config.ron .

RUN chmod +x ./xigma-md

CMD ["./xigma-md"]
