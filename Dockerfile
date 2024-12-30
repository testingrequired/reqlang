# syntax=docker/dockerfile:1

# BUILD ########################################################################
FROM rust:1.75.0-slim-bookworm AS build

WORKDIR /usr/local/src

COPY Cargo.toml Cargo.lock ./
COPY assert-response assert-response/
COPY cli cli/
COPY diagnostics diagnostics/
COPY errors errors/
COPY export export/
COPY integration_tests integration_tests/
COPY parser parser/
COPY reqlang reqlang/
COPY reqlang-client reqlang-client/
COPY reqlang-fetch reqlang-fetch/
COPY reqlang-lsp reqlang-lsp/
COPY span span/
COPY str-idxpos str-idxpos/
COPY types types/

RUN cargo fetch
RUN cargo build --locked --release --package cli

# RUNTIME #####################################################################
FROM debian:bookworm-slim

WORKDIR /usr/local/bin

COPY --from=build /usr/local/src/target/release/reqlang /usr/local/bin/

# Request files can be mounted here
WORKDIR /usr/local/src

ENTRYPOINT ["/usr/local/bin/reqlang"]
