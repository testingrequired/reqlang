# syntax=docker/dockerfile:1

################################################################################

ARG RUST_VERSION=1.75.0
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /app

COPY . .

RUN cargo build --locked --release --package cli

################################################################################

FROM debian:bookworm-slim AS final

RUN groupadd -r user
RUN useradd -r -g user user

WORKDIR /app

RUN chown -R user:user /app

USER user

COPY \
  --from=build \
  /app/target/release/reqlang \
  .

COPY --chown=user \
  --from=build \
  /app/entrypoint.sh \
  .

RUN chmod 755 ./entrypoint.sh

ENTRYPOINT [ "./entrypoint.sh" ]
