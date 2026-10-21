#
# Running `make dev` will set up links for development using symlink to the library source
# Running `make install` will compile a release and install it properly as a release
#

LIBNAME=native_codec_impl
ROOT=$(shell pwd)
UV=uv

#.PHONY: compile
#compile:
#	cargo build && \
#	ln -fs $(ROOT)/target/debug/lib$(LIBNAME).so $(ROOT)/target/debug/$(LIBNAME).so
# && cargo build --release

.PHONY: clearterminal
clearterminal:
	clear && printf '\e[3J'

.PHONY: test
test: clearterminal
	$(UV) run --extra dev pytest

.PHONY: clearterminal dtest
dtest: compile
	PYTHONPATH=$(ROOT):$(ROOT)/target/debug gdb --args python3 test/etf_decode_test.py

.PHONY: docs
docs:
	rm -rf $(ROOT)/docs; \
	$(UV) run --extra docs sphinx-build -M html docs-src/source docs-src/build && \
	mv -f $(ROOT)/docs-src/build/html $(ROOT)/docs && \
	touch $(ROOT)/docs/.nojekyll

#
# Installing for development, and for release
#
.PHONY: install dev
dev:
	$(UV) sync --extra dev

install:
	$(UV) pip install .

# Run Pyre type check (requires: pip install pyre-check):
.PHONY: pyre
pyre:
	pyre check
