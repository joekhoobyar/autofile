.PHONY: image

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
