.PHONY: image version-bump

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
	NEW_VERSION="$$new_version" perl -0pi -e 's/(^version = ")[^"]+(")/$$1$$ENV{NEW_VERSION}$$2/m' api/Cargo.toml; \
	NEW_VERSION="$$new_version" perl -0pi -e 's/(\[\[package\]\]\nname = "autofile-api"\nversion = ")[^"]+(")/$$1$$ENV{NEW_VERSION}$$2/' api/Cargo.lock; \
	NEW_VERSION="$$new_version" perl -0pi -e 's/("name": "autofile-ui",\n\s+"version": ")[^"]+(")/$$1$$ENV{NEW_VERSION}$$2/' ui/package.json; \
	NEW_VERSION="$$new_version" perl -0pi -e 's/("name": "autofile-ui",\n\s+"version": ")[^"]+(")/$$1$$ENV{NEW_VERSION}$$2/g' ui/package-lock.json; \
	echo "Bumped version: $$current_version -> $$new_version"
