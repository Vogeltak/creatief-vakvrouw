FROM rust:1.68.0-bullseye as builder

WORKDIR /usr/src/app
COPY . .

RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates libssl-dev fonts-liberation2 texlive-latex-recommended texlive-latex-extra texlive-xetex pandoc
COPY --from=builder /usr/local/cargo/bin/creatief-vakvrouw /usr/local/bin/creatief-vakvrouw

EXPOSE 1728/tcp

CMD ["creatief-vakvrouw", "server"]
