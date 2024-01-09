FROM rust:slim-buster as builder
WORKDIR /app
RUN chown -R $(whoami) /app
# COPY ./Cargo.toml ./
# COPY ./Cargo.lock ./
# COPY ./src/ ./src/
COPY . .
RUN cargo build --release

FROM cgr.dev/chainguard/glibc-dynamic:latest
ENV PACKAGE=lbtestapi
COPY --from=builder --chown=nonroot:nonroot /app/target/release/${PACKAGE} /usr/local/bin/${PACKAGE}
CMD ["/usr/local/bin/${PACKAGE}"]
