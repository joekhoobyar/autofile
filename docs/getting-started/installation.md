# Installation

Autofile can be installed with Helm on Kubernetes, or run locally with Docker Compose.

## Choose An Installation Method

| Method | Best For | Notes |
| --- | --- | --- |
| [Helm](helm.md) | Kubernetes deployments | Installs Autofile, optional bundled Valkey and RustFS, and a CloudNativePG database by default. |
| [Docker Compose](quick-start.md) | Local development and evaluation | Starts the API, UI, PostgreSQL, Redis, and RustFS from a repository checkout. |

## Helm

Use Helm when deploying Autofile to a Kubernetes cluster. The published chart is available from GHCR as an OCI chart:

```text
oci://ghcr.io/joekhoobyar/charts/autofile
```

See [Helm Installation](helm.md) for the install command and chart-specific notes.

## Docker Compose

Use Docker Compose for local development or a quick evaluation environment:

```bash
docker compose up
```

See [Docker Compose Quick Start](quick-start.md) for the services and local endpoints.
