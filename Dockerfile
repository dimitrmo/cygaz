FROM scratch

COPY target/release/cygaz /

ENV TIMEOUT 600000
ENV HOST    0.0.0.0
ENV PORT    8080

EXPOSE 8080

CMD ["/cygaz"]
