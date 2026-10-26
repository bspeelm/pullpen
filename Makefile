# pullpen — `make check` before every commit (CLAUDE.md).

# ADR-004 is why these limits exist and have this shape. The numbers are not
# repeated in prose here; they are the lines below, and a test fails if any
# document quotes them differently.
MAX_BINARY_BYTES   := 10485760
MAX_CODE_LINES     := 9000
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

.PHONY: all build test lint fmt budgets golden keymap check clean install-binary release release-tag

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

# Named for what it installs, as bothy's is: the release binary, into
# ~/.local/bin, which is on PATH on most systems and needs no root.
install-binary: build
	install -Dm755 $(BINARY) $(HOME)/.local/bin/pullpen
	@echo "installed to ~/.local/bin/pullpen"

# Step one of a release: open the PR that bumps the version.
#
#     make release VERSION=0.2.0
#
# main takes changes only through a pull request, and the release workflow
# refuses a tag that disagrees with Cargo.toml, so the bump lands on main
# before the tag exists.
release:
	@test -n "$(VERSION)" || { echo "usage: make release VERSION=x.y.z"; exit 1; }
	@echo "$(VERSION)" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$$' || { echo "VERSION must be x.y.z"; exit 1; }
	@git diff --quiet || { echo "working tree is dirty; commit first"; exit 1; }
	@test "$$(git rev-parse --abbrev-ref HEAD)" = main || { echo "release starts from main"; exit 1; }
	@! git rev-parse -q --verify "refs/tags/v$(VERSION)" >/dev/null || { echo "v$(VERSION) already exists"; exit 1; }
	@command -v gh >/dev/null || { echo "gh is not installed"; exit 1; }
	git switch -c release/$(VERSION)
	sed -i.bak '/^\[workspace.package\]/,/^\[/s/^version = ".*"/version = "$(VERSION)"/' Cargo.toml && rm Cargo.toml.bak
	cargo update --workspace --quiet
	$(MAKE) check
	git commit -am "build: $(VERSION)"
	git push -u origin release/$(VERSION)
	gh pr create --base main --head release/$(VERSION) --title "build: $(VERSION)" \
	    --body "Bumps the version to $(VERSION). Merging this, then \`make release-tag\`, cuts the release."

# Step two: tag the merged bump. The tag is what the release workflow
# watches. Branch rules do not cover tags, so this needs no PR.
release-tag:
	@test "$$(git rev-parse --abbrev-ref HEAD)" = main || { echo "tag from main"; exit 1; }
	@git diff --quiet || { echo "working tree is dirty"; exit 1; }
	git fetch --quiet origin main
	@test "$$(git rev-parse HEAD)" = "$$(git rev-parse origin/main)" || { echo "main is not level with origin/main"; exit 1; }
	@v=$$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml); \
	! git rev-parse -q --verify "refs/tags/v$$v" >/dev/null || { echo "v$$v is already tagged; was the bump merged?"; exit 1; }; \
	git tag "v$$v" && git push origin "v$$v" && echo "pushed v$$v; the release workflow is building it"
