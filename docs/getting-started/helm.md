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

Install the latest published chart version:

```bash
helm install autofile oci://ghcr.io/joekhoobyar/charts/autofile
```

Upgrade an existing release, or install it if it does not exist:

```bash
helm upgrade --install autofile oci://ghcr.io/joekhoobyar/charts/autofile
```

Pass custom values with `-f`:

```bash
helm upgrade --install autofile oci://ghcr.io/joekhoobyar/charts/autofile \
  -f values.yaml
```

For repeatable installs, pin `--version` to a published chart version. Discover the latest published version with:

```bash
helm show chart oci://ghcr.io/joekhoobyar/charts/autofile
```

Then install that version explicitly:

```bash
HELM_CHART_VERSION="$(helm show chart oci://ghcr.io/joekhoobyar/charts/autofile | yq '.version')"
helm upgrade --install autofile oci://ghcr.io/joekhoobyar/charts/autofile \
  --version "${HELM_CHART_VERSION}"
```

Published chart versions are also indexed on [Artifact Hub](https://artifacthub.io/), including per-version changes. Each release publishes a SLSA build provenance attestation for the chart OCI artifact, verifiable with:

```bash
gh attestation verify oci://ghcr.io/joekhoobyar/charts/autofile@<digest> --owner joekhoobyar
```

## Database Modes

By default, the chart creates a CloudNativePG `Cluster` and reads the generated database URI from Secret `<database.cnpg.name>-app`, key `uri`.

Use an external PostgreSQL database by setting `database.mode=external` and supplying `database.url`:

```yaml
database:
  mode: external
  url: postgres://user:password@postgres.example.com:5432/autofile
```

## Virus Scanning

The chart deploys a bundled ClamAV scanner by default (`clamav.enabled=true`, `virusScanning.provider=clamav`). Uploads are not scanned until an administrator enables virus scanning in application settings. To omit the scanner, set `virusScanning.provider=disabled` and `clamav.enabled=false`; to use an external `clamd` endpoint instead, set `virusScanning.provider=external` with `virusScanning.clamavHost`/`clamavPort`. See [Configuration](../admin/configuration.md#virus-scanning) for details.

## Full Chart Reference

The full chart README contains all values, local chart development commands, and dependency notes:

[Autofile Helm Chart README](https://github.com/joekhoobyar/autofile/tree/main/charts/autofile)
