FROM rust:1.56 as build

RUN curl https://sh.rustup.rs -sSf | sh -s -- --no-modify-path --default-toolchain none -y
RUN rustup component add rustfmt

WORKDIR /usr/src

# Use a dummy entrypoint to build each crate, that way dependencies can be cached
# without code changes causing a complete rebuild.
# TODO all these COPY commands add additional layers to the image. I'm not sure
# how big they are but for the distributed images we likely want to use
# a simpler Dockerfile.
COPY Cargo.toml .
RUN mkdir -p pcli/src pd/src &&\
    echo "fn main() {}" > pcli/src/main.rs &&\
    echo "fn main() {}" > pd/src/main.rs
COPY pcli/build.rs ./pcli
COPY pcli/Cargo.toml ./pcli
COPY pd/Cargo.toml ./pd
COPY pd/build.rs ./pd
# TODO If the protobuf definitions, crypto, or wallet implementations change,
# there will be a complete rebuild. This is maybe possible to avoid.
COPY proto ./proto
COPY crypto ./crypto
COPY decaf377-fmd ./decaf377-fmd
COPY decaf377-ka ./decaf377-ka
COPY wallet ./wallet
COPY .git ./.git
# Sorry about all that mess ^, but it's worth it during development.

# Fetch dependencies in a separate layer, so that they can be cached.
COPY Cargo.lock .
RUN cargo fetch --locked

RUN cargo build --bin pd --release --frozen

# Remove the cached builds of internal packages.
RUN rm -rf pcli pd crypto wallet config

# Copy the repo source now that dependencies have been built and cached.
COPY . .

RUN cargo build --bin pd --release --frozen && \
    mkdir -p /out && \
    mv target/release/pd /out/pd

# Install the penumbra daemon into the runtime image.

# TODO(eliza): it would be nice to be able to run the Penumbra daemon in a
# `scratch` image rather than Debian or Alpine. However, then we'd have to build
# with a statically linked libc (read: musl), and musl's malloc exhibits
# pathologically poor performance for Tokio applications...
FROM debian:buster-slim as runtime
ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL
WORKDIR /penumbra
COPY --from=build /out/pd /usr/bin/pd
ENV RUST_LOG=warn,pd=info,penumbra=info
CMD [ "/usr/bin/pd" ]
