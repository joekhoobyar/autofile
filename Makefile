.PHONY: image changelog-release changelog-unreleased version-bump release-push release

image:
	@set -e; \
	git_sha="$${GIT_SHA:-$$(git rev-parse --short HEAD)}"; \
	if [ -z "$$git_sha" ]; then \
		echo "Unable to determine GIT_SHA. Set GIT_SHA explicitly." >&2; \
		exit 1; \
	fi; \
	if [ -n "$$(git status --porcelain)" ]; then \
		case "$$git_sha" in \
			*-dirty) ;; \
			*) git_sha="$${git_sha}-dirty" ;; \
		esac; \
	fi; \
	echo "GIT_SHA=$$git_sha docker buildx bake --push $(TARGET) $(ARGS)"; \
	GIT_SHA="$$git_sha" docker buildx bake --push $(TARGET) $(ARGS)

release: changelog-release
	@set -e; \
	current_version="$$(perl -ne 'print $$1 and exit if /^version = "([0-9]+\.[0-9]+\.[0-9]+)"/' api/Cargo.toml)"; \
	git commit -m "prepare release v$$current_version" CHANGELOG.md
	$(MAKE) release-push
	$(MAKE) version-bump
	$(MAKE) changelog-unreleased
	git commit -m 'version bump for next dev cycle' \
		CHANGELOG.md api/Cargo.toml api/Cargo.lock charts/autofile/Chart.yaml ui/package.json ui/package-lock.json && \
	git push

changelog-release:
	@set -e; \
	current_version="$$(perl -ne 'print $$1 and exit if /^version = "([0-9]+\.[0-9]+\.[0-9]+)"/' api/Cargo.toml)"; \
	if [ -z "$$current_version" ]; then \
		echo "Unable to determine current version from api/Cargo.toml" >&2; \
		exit 1; \
	fi; \
	if [ ! -f CHANGELOG.md ]; then \
		echo "CHANGELOG.md not found" >&2; \
		exit 1; \
	fi; \
	date="$$(date -u +%Y-%m-%d)"; \
	if ! grep -qxF '## [Unreleased]' CHANGELOG.md; then \
		echo "CHANGELOG.md must contain a '## [Unreleased]' heading" >&2; \
		exit 1; \
	fi; \
	if grep -qxF "## [$$current_version]" CHANGELOG.md || grep -qF "## [$$current_version] - " CHANGELOG.md; then \
		echo "CHANGELOG.md already contains a section for $$current_version" >&2; \
		exit 1; \
	fi; \
	perl -0pi -e 's/^## \[Unreleased\]$$/## ['"$$current_version"'] - '"$$date"'/m' CHANGELOG.md; \
	if ! grep -qxF "## [$$current_version] - $$date" CHANGELOG.md; then \
		echo "Failed to prepare CHANGELOG.md for $$current_version" >&2; \
		exit 1; \
	fi; \
	echo "Prepared CHANGELOG.md for $$current_version"

changelog-unreleased:
	@set -e; \
	if [ ! -f CHANGELOG.md ]; then \
		echo "CHANGELOG.md not found" >&2; \
		exit 1; \
	fi; \
	if grep -qxF '## [Unreleased]' CHANGELOG.md; then \
		echo "CHANGELOG.md already contains an Unreleased section" >&2; \
		exit 1; \
	fi; \
	perl -0pi -e 's/^(# Changelog\n(?:.*\n)*?and this project adheres to \[Semantic Versioning\]\(https:\/\/semver\.org\/spec\/v2\.0\.0\.html\)\.\n)\n/$${1}\n## [Unreleased]\n\n/m' CHANGELOG.md; \
	if ! grep -qxF '## [Unreleased]' CHANGELOG.md; then \
		echo "Failed to add CHANGELOG.md Unreleased section" >&2; \
		exit 1; \
	fi; \
	echo "Added CHANGELOG.md Unreleased section"

release-push:
	@set -e; \
	current_version="$$(perl -ne 'print $$1 and exit if /^version = "([0-9]+\.[0-9]+\.[0-9]+)"/' api/Cargo.toml)"; \
	if [ -z "$$current_version" ]; then \
		echo "Unable to determine current version from api/Cargo.toml" >&2; \
		exit 1; \
	fi; \
	git tag "v$$current_version" && \
	git push origin "v$$current_version"

version-bump:
	@set -e; \
	current_version="$$(perl -ne 'print $$1 and exit if /^version = "([0-9]+\.[0-9]+\.[0-9]+)"/' api/Cargo.toml)"; \
	if [ -z "$$current_version" ]; then \
		echo "Unable to determine current version from api/Cargo.toml" >&2; \
		exit 1; \
	fi; \
	new_version="$${VERSION:-}"; \
	if [ -z "$$new_version" ]; then \
		major="$${current_version%%.*}"; \
		rest="$${current_version#*.}"; \
		minor="$${rest%%.*}"; \
		patch="$${rest#*.}"; \
		new_version="$$major.$$minor.$$((patch + 1))"; \
	fi; \
	if ! perl -e 'exit !(shift =~ /^\d+\.\d+\.\d+$$/)' "$$new_version"; then \
		echo "VERSION must be semver: MAJOR.MINOR.PATCH" >&2; \
		exit 1; \
	fi; \
	command -v yq >/dev/null || { \
		echo "yq is required to update charts/autofile/Chart.yaml" >&2; \
		exit 1; \
	}; \
	NEW_VERSION="$$new_version" perl -0pi -e 's/(^version = ")[^"]+(")/$$1$$ENV{NEW_VERSION}$$2/m' api/Cargo.toml; \
	cargo update --manifest-path api/Cargo.toml --package autofile-api --offline; \
	npm --prefix ui version "$$new_version" --no-git-tag-version; \
	NEW_VERSION="$$new_version" yq -i '.version = strenv(NEW_VERSION) | .appVersion = "v" + strenv(NEW_VERSION)' charts/autofile/Chart.yaml; \
	echo "Bumped version: $$current_version -> $$new_version"
