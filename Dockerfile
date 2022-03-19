FROM scratch

COPY target/release/cygaz /

EXPOSE 8080

CMD ["/cygaz"]
