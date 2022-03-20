VERSION=$(shell cargo pkgid | cut -d\# -f2 | cut -d: -f2)

.PHONY: prod
prod:
	cargo build --release
	upx -9 target/release/cygaz
	ls -lah target/release/cygaz

.PHONY: run
run: prod
	RUST_LOG=cygaz=debug \
		./target/release/cygaz

.PHONY: docker-build
docker-build:
	docker build --squash -t cygaz:${VERSION} .

.PHONY: docker-run
docker-run:
	docker run --rm -it -p 18080:8080 -e RUST_LOG=cygaz=debug cygaz:${VERSION}
