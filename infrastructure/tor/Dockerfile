FROM alpine:latest
RUN apk add --no-cache tor

# Set up directories with correct ownership and permissions
RUN mkdir -p /var/lib/tor/ignoto_hidden_service && \
    chmod 700 /var/lib/tor/ignoto_hidden_service && \
    mkdir -p /var/lib/tor/run && \
    chmod 700 /var/lib/tor/run && \
    chown -R tor:tor /var/lib/tor

USER tor
ENTRYPOINT ["tor"]
CMD ["-f", "/etc/tor/torrc"]
