# Helm Installation

Use the Helm chart to deploy Autofile to Kubernetes.

## Prerequisites

- Kubernetes cluster
- Helm 3
- CloudNativePG operator for the default `database.mode=cnpg`, or an external PostgreSQL database when using `database.mode=external`
- A default StorageClass, or explicit storage values for CloudNativePG and RustFS

## Install From GHCR

The chart is published as an OCI artifact:

```text
oci://ghcr.io/joekhoobyar/charts/autofile
```

Install a published chart version:

```bash
CHART_VERSION="<chart-version>"
helm install autofile oci://ghcr.io/joekhoobyar/charts/autofile \
  --version "${CHART_VERSION}"
```

Upgrade an existing release, or install it if it does not exist:

```bash
CHART_VERSION="<chart-version>"
helm upgrade --install autofile oci://ghcr.io/joekhoobyar/charts/autofile \
  --version "${CHART_VERSION}"
```

Pass custom values with `-f`:

```bash
CHART_VERSION="<chart-version>"
helm upgrade --install autofile oci://ghcr.io/joekhoobyar/charts/autofile \
  --version "${CHART_VERSION}" \
  -f values.yaml
```

## Database Modes

By default, the chart creates a CloudNativePG `Cluster` and reads the generated database URI from Secret `<database.cnpg.name>-app`, key `uri`.

Use an external PostgreSQL database by setting `database.mode=external` and supplying `database.url`:

```yaml
database:
  mode: external
  url: postgres://user:password@postgres.example.com:5432/autofile
```

## Full Chart Reference

The full chart README contains all values, local chart development commands, and dependency notes:

[Autofile Helm Chart README](https://github.com/joekhoobyar/autofile/tree/main/charts/autofile)
