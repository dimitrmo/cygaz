VERSION=$(shell cargo pkgid | cut -d\# -f2 | cut -d: -f2)

.PHONY: tag
tag:
	git tag -a v${VERSION}

.PHONY: prod
prod:
	cargo build --release
	ls -lah target/release/cygaz
	upx -9 target/release/cygaz
	ls -lah target/release/cygaz

.PHONY: run
run: prod
	RUST_LOG=cygaz=debug \
		./target/release/cygaz

.PHONY: patch
patch:
	curl -X PATCH http://localhost:8080/prices/1/refresh

.PHONY: docker-build
docker-build:
	docker build --squash -t cygaz:${VERSION} .

.PHONY: docker-push
docker-push:
	docker tag cygaz:${VERSION} rg.fr-par.scw.cloud/dimitrmo/cygaz:${VERSION}
	docker push rg.fr-par.scw.cloud/dimitrmo/cygaz:${VERSION}

.PHONY: docker-run
docker-run:
	docker run --rm -it -p 18080:8080 -e RUST_LOG=cygaz=debug cygaz:${VERSION}
