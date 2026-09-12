---
hide:
  - navigation
  - toc
---

# OpenAPI Specification

The interactive reference below is rendered from the `openapi.json` asset attached to the latest GitHub release, baked into this site when the docs were built. It describes every endpoint under `/api/v1` and is generated directly from the Rust API source, so it stays in sync without committing a large generated file to the repository.

[Download the raw `openapi.json` for the latest release](https://github.com/joekhoobyar/autofile/releases/latest/download/openapi.json)

!!! note "Versioning and offline use"
    This page reflects the latest *released* version, not unreleased changes on `main`. The authoritative contract for your deployment is always the running API itself: `GET /api/v1/openapi.json` with interactive Swagger UI at `/api/docs`. In air-gapped environments, fetch the spec from your own server instead of the release download link above:

    ```bash
    curl -s http://localhost:8000/api/v1/openapi.json -o openapi.json
    ```

<style>
  /* Page-scoped full width: the theme caps content at a 61rem grid for
     prose readability, but the API reference needs the whole viewport. */
  .md-grid {
    max-width: initial;
  }
</style>
<div id="redoc-container"></div>
<script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
<script>
  Redoc.init(
    '../../assets/openapi/openapi.json',
    { scrollYOffset: 64 },
    document.getElementById('redoc-container')
  );
</script>
