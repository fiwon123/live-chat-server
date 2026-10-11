# syntax=docker/dockerfile:1.6
FROM rust:1.83-bookworm

ARG GH_CLI_VERSION=v2.102.0
ARG TARGETARCH
ARG HOST_UID=1000
ARG HOST_GID=1000

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev postgresql-client curl ca-certificates git zsh \
    && rm -rf /var/lib/apt/lists/*

RUN if [ -z "$TARGETARCH" ]; then \
      case "$(uname -m)" in x86_64|amd64) TARGETARCH=amd64 ;; aarch64|arm64) TARGETARCH=arm64 ;; *) exit 1 ;; esac; \
    fi && curl -fsSL "https://github.com/cli/cli/releases/download/${GH_CLI_VERSION}/gh_${GH_CLI_VERSION#v}_linux_${TARGETARCH}.tar.gz" -o /tmp/gh.tar.gz && \
    tar -xzf /tmp/gh.tar.gz -C /tmp && mv "/tmp/gh_${GH_CLI_VERSION#v}_linux_${TARGETARCH}/bin/gh" /usr/local/bin/gh && rm -rf /tmp/*gh* && gh --version

RUN if ! getent group ${HOST_GID} >/dev/null; then groupadd --gid ${HOST_GID} devuser; fi && \
    if ! getent passwd ${HOST_UID} >/dev/null; then useradd --uid ${HOST_UID} --gid ${HOST_GID} --create-home --shell /usr/bin/zsh devuser; fi && \
    mkdir -p /home/devuser && chown -R ${HOST_UID}:${HOST_GID} /home/devuser && git config --system --add safe.directory /workspace

ENV CARGO_HOME=/usr/local/cargo RUSTUP_HOME=/usr/local/rustup PATH=/usr/local/cargo/bin:$PATH
RUN chown -R ${HOST_UID}:${HOST_GID} /usr/local/cargo /usr/local/rustup

COPY scripts/dev-sandbox-rc.sh /etc/profile.d/00-dev-sandbox.sh
RUN chsh -s /usr/bin/zsh root 2>/dev/null || true && \
    for f in /root/.zshrc /root/.bashrc /root/.zprofile /home/devuser/.zshrc /home/devuser/.bashrc /home/devuser/.zprofile; do \
      printf '%s\n' '[ -f /etc/profile.d/00-dev-sandbox.sh ] && . /etc/profile.d/00-dev-sandbox.sh' >> "$f"; done && \
    chown ${HOST_UID}:${HOST_GID} /home/devuser/.bashrc /home/devuser/.zshrc /home/devuser/.zprofile

USER devuser
WORKDIR /workspace
CMD ["sleep", "infinity"]
