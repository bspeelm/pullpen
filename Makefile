# pullpen — `make check` before every commit (CLAUDE.md).

# ADR-004 is why these limits exist and have this shape. The numbers are not
# repeated in prose here; they are the lines below, and a test fails if any
# document quotes them differently.
MAX_BINARY_BYTES   := 10485760
MAX_CODE_LINES     := 8000
MAX_COMMENT_RATIO  := 30
MAX_DOC_RATIO      := 75
DOC_RATIO_FROM     := 2000

BINARY := target/release/pullpen

# Code is the non-test Rust under crates/*/src; unit tests live in tests.rs
# files, so a filename filter is enough to tell them apart.
SOURCES   := $(shell find crates -path '*/src/*' -name '*.rs' -not -name 'tests.rs')
# Prose is every markdown file except the design, which is delivered rather
# than written here, and history, which is append-only by design.
LIVE_DOCS := $(shell find . -name '*.md' -not -path './target/*' -not -path './.git/*' \
                     -not -path './design/*' -not -path './docs/history/*')

.PHONY: all build test lint fmt budgets golden keymap check clean

all: check

build:
	cargo build --release --locked

test:
	cargo test --workspace --locked

fmt:
	cargo fmt --all

lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --locked -- -D warnings

# Regenerates tests/golden from design/tui.js (ADR-002). Needs node; nothing
# else does, because the tests read the committed output.
golden:
	node scripts/gen-golden.mjs

# Regenerates docs/keymap.md from the keymap table.
keymap:
	PULLPEN_BLESS=1 cargo test -q -p pullpen-ui --lib keymap::tests::docs_keymap_md

budgets: build
	@size=$$(wc -c < $(BINARY)); \
	echo "binary:   $$size bytes (budget $(MAX_BINARY_BYTES))"; \
	if [ $$size -gt $(MAX_BINARY_BYTES) ]; then echo "over the binary budget"; exit 1; fi
	@code=$$(cat $(SOURCES) | awk '/^[[:space:]]*\/\*/{b=1} b{if(/\*\//)b=0; next} !/^[[:space:]]*\/\// && NF' | wc -l); \
	comments=$$(cat $(SOURCES) | awk '/^[[:space:]]*\/\*/{b=1} b{c++; if(/\*\//)b=0; next} /^[[:space:]]*\/\//{c++} END{print c+0}'); \
	tenths=$$(( comments * 1000 / (code > 0 ? code : 1) )); \
	echo "code:     $$code lines (budget $(MAX_CODE_LINES))"; \
	echo "comments: $$comments lines, $$(( tenths / 10 )).$$(( tenths % 10 ))% of code (budget $(MAX_COMMENT_RATIO)%)"; \
	if [ $$code -gt $(MAX_CODE_LINES) ]; then echo "over the code budget"; exit 1; fi; \
	if [ $$(( comments * 100 )) -gt $$(( $(MAX_COMMENT_RATIO) * code )) ]; then echo "over the comment budget"; exit 1; fi; \
	docs=$$(cat $(LIVE_DOCS) | wc -l); \
	dtenths=$$(( docs * 1000 / (code > 0 ? code : 1) )); \
	if [ $$code -lt $(DOC_RATIO_FROM) ]; then \
	    echo "prose:    $$docs lines, $$(( dtenths / 10 )).$$(( dtenths % 10 ))% of code (budget $(MAX_DOC_RATIO)%, enforced from $(DOC_RATIO_FROM) lines of code)"; \
	else \
	    echo "prose:    $$docs lines, $$(( dtenths / 10 )).$$(( dtenths % 10 ))% of code (budget $(MAX_DOC_RATIO)%)"; \
	    if [ $$(( docs * 100 )) -gt $$(( $(MAX_DOC_RATIO) * code )) ]; then echo "over the prose budget"; exit 1; fi; \
	fi

check: lint test budgets

clean:
	cargo clean
