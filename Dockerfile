# Based on https://kerkour.com/rust-small-docker-image
FROM rust:1.66 AS builder

WORKDIR /server/server

# Create appuser
ENV USER=server
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

COPY ./ /server/server

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /server

# Copy our build
COPY --from=builder /server/server/target/x86_64-unknown-linux-musl/release/server ./

# Use an unprivileged user.
USER server:server

CMD ["/server/server"]