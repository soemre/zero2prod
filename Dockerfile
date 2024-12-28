ARG APP_BIN=zero2prod

FROM clux/muslrust:1.83.0-stable AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . . 
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE=true
ARG APP_BIN
RUN cargo build -r --bin "$APP_BIN"

FROM scratch AS runtime
WORKDIR /app
COPY config config
ARG APP_BIN
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/"$APP_BIN" run
ENV APP_ENV=production
CMD ["/app/run"]
