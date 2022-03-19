.PHONY: prod
prod:
	cargo build --release
	upx -9 target/release/cygaz
	ls -lah target/release/cygaz

.PHONY: run
run: prod
	RUST_LOG=cygaz=info \
		./target/release/cygaz

.PHONY: docker-build
docker-build:
	docker build --squash -t cygaz:latest .

.PHONY: docker-run
docker-run:
	docker run --rm -it -p 18080:8080 cygaz:latest
