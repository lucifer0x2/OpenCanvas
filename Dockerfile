# syntax=docker/dockerfile:1
# OpenCanvas in one image: Rust server and layout-step, Node render worker with Playwright
# Chromium, built renderer bundle and viewer. Build: docker build -t opencanvas:local .
ARG RUST_IMAGE=rust:1.99.1-slim-trixie
ARG NODE_IMAGE=node:24.21.0-trixie-slim

FROM ${RUST_IMAGE} AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
# opencanvas-storage embeds the bundled font and its licence (include_bytes!).
COPY renderer/fonts renderer/fonts
# layout-step stays in the dev profile: the worker (worker/pipeline.mjs) and the pipeline
# fingerprint (crates/server/src/fingerprint.rs) look for <workspace>/target/debug/layout-step,
# and it is the build the native acceptance runs verified. The server itself is a release build.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --locked -p opencanvas-layout --bin layout-step \
    && cargo build --locked --release -p opencanvas-server \
    && mkdir /out \
    && cp target/debug/layout-step target/release/opencanvas-server /out/

FROM ${NODE_IMAGE} AS web
WORKDIR /src
COPY package.json package-lock.json tsconfig.json ./
COPY web/package.json web/package-lock.json web/
RUN --mount=type=cache,target=/root/.npm npm ci && npm --prefix web ci
COPY renderer renderer
COPY web web
RUN npm run build:renderer && npm --prefix web run build

FROM ${NODE_IMAGE} AS runtime
ENV PLAYWRIGHT_BROWSERS_PATH=/app/.runtime/browsers
WORKDIR /app
# The worker and the fingerprint import `playwright`; nothing else from node_modules runs here.
COPY --from=web /src/node_modules/playwright node_modules/playwright
COPY --from=web /src/node_modules/playwright-core node_modules/playwright-core
# chromium.launch({headless:true}) uses the headless shell, so the full browser is not installed.
RUN node node_modules/playwright/cli.js install --with-deps --only-shell chromium \
    && rm -rf /var/lib/apt/lists/* /tmp/*
# Lockfiles are hashed into the pipeline identity; worker, renderer bundle and font are served or
# executed from the workspace (/app).
COPY package.json package-lock.json Cargo.lock ./
COPY worker worker
COPY renderer/fonts renderer/fonts
COPY --from=web /src/renderer/dist renderer/dist
COPY --from=web /src/web/dist web/dist
COPY --from=rust /out/layout-step target/debug/layout-step
COPY --from=rust /out/opencanvas-server /usr/local/bin/opencanvas-server
RUN mkdir /data && chown node:node /data
USER node
VOLUME /data
EXPOSE 3847
# The server stops gracefully on SIGINT (Ctrl-C) only.
STOPSIGNAL SIGINT
CMD ["opencanvas-server", "--workspace", "/app", "--data-root", "/data", "--host", "0.0.0.0", "--port", "3847"]
