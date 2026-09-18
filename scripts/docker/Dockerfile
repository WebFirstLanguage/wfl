# The context contains only the tested static Linux binary, LICENSE, and .wflcfg.
# Base digest: docker-library/repo-info, repos/debian/remote/bookworm-slim.md.
FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171

ARG VERSION
ARG REVISION
ARG TARGETARCH

LABEL org.opencontainers.image.title="WFL script test runtime" \
      org.opencontainers.image.description="Run WFL programs and built-in describe/test/expect suites" \
      org.opencontainers.image.source="https://github.com/WebFirstLanguage/wfl" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}"

# TLS uses rustls and the supplied musl binary is static. The CA bundle is
# needed for scripts that make HTTPS requests; /bin/sh comes with Debian.
RUN test "$TARGETARCH" = amd64 \
    && apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 wfl \
    && useradd --uid 10001 --gid wfl --home-dir /var/lib/wfl --create-home --shell /bin/sh wfl \
    && install -d -o wfl -g wfl /work \
    && install -d /etc/wfl /usr/share/licenses/wfl

COPY --chmod=0755 wfl /usr/local/bin/wfl
COPY --chmod=0644 .wflcfg /etc/wfl/wfl.cfg
COPY --chmod=0644 LICENSE /usr/share/licenses/wfl/LICENSE

# Fail the build if the staged binary and the publication version disagree.
RUN test -n "$VERSION" \
    && test "$(wfl --version)" = "WebFirst Language (WFL) version $VERSION"

ENV HOME=/var/lib/wfl \
    LANG=C.UTF-8
WORKDIR /work
USER 10001:10001

ENTRYPOINT ["/usr/local/bin/wfl"]
CMD ["--help"]
