FROM alpine:latest
RUN apk add --no-cache git
COPY gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]
