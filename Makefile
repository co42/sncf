.PHONY: build release test clean

VERSION ?= $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
HOMEBREW_TAP := $(HOME)/self/homebrew-tap
REPO := co42/sncf

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

release:
	@if [ -z "$(VERSION)" ]; then echo "Could not determine version"; exit 1; fi
	@if [ ! -d "$(HOMEBREW_TAP)" ]; then echo "Homebrew tap not found at $(HOMEBREW_TAP)"; exit 1; fi
	@if git rev-parse "v$(VERSION)" >/dev/null 2>&1; then echo "Tag v$(VERSION) already exists"; exit 1; fi
	@echo "=== Preparing release v$(VERSION) ==="
	@sed -i '' 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@cargo generate-lockfile
	@git add -A
	@git commit -m "chore: release v$(VERSION)" || true
	@git tag "v$(VERSION)"
	@git push && git push --tags
	@echo ""
	@echo "=== Waiting for GitHub Actions to build release ==="
	@sleep 10
	@RUN_ID=$$(gh run list -R $(REPO) --branch v$(VERSION) --limit 1 --json databaseId -q '.[0].databaseId') && \
		echo "Watching workflow run $$RUN_ID..." && \
		gh run watch $$RUN_ID -R $(REPO) --exit-status || (echo "Release build failed!" && exit 1)
	@echo ""
	@echo "=== Updating homebrew formula ==="
	@SHA_ARM=$$(gh release view v$(VERSION) -R $(REPO) --json assets -q '.assets[] | select(.name | contains("aarch64-apple-darwin")) | .digest' | sed 's/sha256://') && \
		SHA_X86_MAC=$$(gh release view v$(VERSION) -R $(REPO) --json assets -q '.assets[] | select(.name | contains("x86_64-apple-darwin")) | .digest' | sed 's/sha256://') && \
		SHA_LINUX=$$(gh release view v$(VERSION) -R $(REPO) --json assets -q '.assets[] | select(.name | contains("x86_64-unknown-linux-gnu")) | .digest' | sed 's/sha256://') && \
		echo "SHA256 aarch64-apple-darwin: $$SHA_ARM" && \
		echo "SHA256 x86_64-apple-darwin:  $$SHA_X86_MAC" && \
		echo "SHA256 x86_64-unknown-linux: $$SHA_LINUX" && \
		sed -i '' 's/version ".*"/version "$(VERSION)"/' $(HOMEBREW_TAP)/Formula/sncf.rb && \
		sed -i '' 's|sncf-v[0-9.]*-aarch64|sncf-v$(VERSION)-aarch64|g' $(HOMEBREW_TAP)/Formula/sncf.rb && \
		sed -i '' 's|sncf-v[0-9.]*-x86_64|sncf-v$(VERSION)-x86_64|g' $(HOMEBREW_TAP)/Formula/sncf.rb && \
		sed -i '' 's|download/v[^/]*/sncf|download/v$(VERSION)/sncf|g' $(HOMEBREW_TAP)/Formula/sncf.rb && \
		awk -v arm="$$SHA_ARM" -v x86="$$SHA_X86_MAC" -v linux="$$SHA_LINUX" \
			'BEGIN{n=0} /sha256/{n++;if(n==1)sub(/sha256 "[^"]*"/,"sha256 \""arm"\"");if(n==2)sub(/sha256 "[^"]*"/,"sha256 \""x86"\"");if(n==3)sub(/sha256 "[^"]*"/,"sha256 \""linux"\"")} {print}' \
			$(HOMEBREW_TAP)/Formula/sncf.rb > $(HOMEBREW_TAP)/Formula/sncf.rb.tmp && \
		mv $(HOMEBREW_TAP)/Formula/sncf.rb.tmp $(HOMEBREW_TAP)/Formula/sncf.rb
	@cd $(HOMEBREW_TAP) && git add -A && git commit -m "sncf $(VERSION)" && git push
	@echo ""
	@echo "=== Release v$(VERSION) complete! ==="
	@echo "  - Tagged and pushed sncf"
	@echo "  - GitHub Actions built binaries"
	@echo "  - Updated and pushed homebrew-tap"
