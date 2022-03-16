#!/bin/bash

ROOT=$(git rev-parse --show-toplevel)

source ${ROOT}/scripts/env.sh

cargo install mdbook mdbook-plantuml

(
	cd ${ROOT}/docs
	mdbook build

	cd ${ROOT}/rmm
	cargo doc --lib --no-deps -p rmm-core

	cd ..
	cp -R out/doc out/book/crates
)
