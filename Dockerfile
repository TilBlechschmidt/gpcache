FROM alpine

COPY target/x86_64-unknown-linux-musl/release/gpcache /gpcache

EXPOSE 3000

ENTRYPOINT /gpcache