FROM rust:1.85.0-slim-bullseye AS build
WORKDIR /app/
COPY ./ /app/
RUN rustup toolchain install stable-x86_64-unknown-linux-gnu
RUN rm -Rf target
RUN cargo build --release

FROM debian:bullseye-slim AS release
WORKDIR /app/
COPY --from=build /app/target/release/portfolio-site-backend /app/portfolio-site-backend
EXPOSE 8080
ENTRYPOINT ["/app/portfolio-site-backend"]


FROM rust:1.85.0-slim-bullseye AS dev
WORKDIR /app/
COPY ./ /app/
RUN rm -Rf target && rm -Rf .git && rm -Rf .github
RUN rustup toolchain install stable-x86_64-unknown-linux-gnu
RUN cargo install --locked cargo-watch@8.5.3
RUN cargo build
EXPOSE 8080
ENTRYPOINT ["cargo", "watch", "-x", "run"]