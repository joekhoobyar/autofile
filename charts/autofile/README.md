# Autofile Helm Chart

This chart deploys Autofile's API and UI, with optional bundled Valkey, RustFS, and CloudNativePG resources.

By default, the chart installs:

- Autofile API Deployment, Service, ServiceAccount, and environment Secret
- Autofile UI Deployment and Service
- A CloudNativePG `Cluster` resource
- Valkey through the `valkey` dependency
- RustFS through the `rustfs` dependency
- A parent-managed Valkey users Secret with a generated default-user password
- A parent-managed RustFS Secret with generated S3-compatible credentials
- An API init container that waits for RustFS and creates the configured bucket

## Prerequisites

- Kubernetes cluster
- Helm 3
- The CloudNativePG operator for the default `database.mode=cnpg`, or an external PostgreSQL database reachable from the API when using `database.mode=external`
- A default StorageClass, or RustFS storage values configured for your cluster

## Install

Update dependencies before installing from a checkout:

```bash
helm dependency update charts/autofile
```

Install with the default CNPG database and bundled Valkey and RustFS dependencies:

```bash
helm install autofile charts/autofile
```

Upgrade an existing release:

```bash
helm upgrade autofile charts/autofile
```

Validate locally:

```bash
helm lint charts/autofile --set database.mode=external --set database.url=postgres://user:pass@db:5432/autofile
helm template autofile charts/autofile --api-versions postgresql.cnpg.io/v1/Cluster
```

The default `database.mode=cnpg` validates that the CloudNativePG `Cluster` API is available. For offline renders, pass `--api-versions postgresql.cnpg.io/v1/Cluster`; when rendering against a cluster, Helm discovers this from an installed CloudNativePG operator.

## Object Storage Modes

Autofile stores document files in S3-compatible object storage. The chart-level `objectStorage.mode` controls how object storage is wired into the API.

| Mode | Behavior |
| --- | --- |
| `rustfs` | Bundled RustFS is used. The chart generates RustFS credentials, creates the RustFS credential Secret, sets `AWS_ENDPOINT_URL_S3`, and adds an API init container to create the bucket. |
| `external` | An external S3-compatible service is used. The chart emits `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` from `secrets.*`. Configure any endpoint through `api.extraEnv`. |
| `s3` | AWS S3 is used. Static AWS credentials are optional and are emitted only when both `secrets.AWS_ACCESS_KEY_ID` and `secrets.AWS_SECRET_ACCESS_KEY` are non-empty. |

For `rustfs` mode, keep `rustfs.enabled=true`. For `external` or `s3`, set `rustfs.enabled=false` unless you intentionally want to deploy RustFS without using it for Autofile.

## Database Modes

The chart-level `database.mode` controls how PostgreSQL is wired into the API.

| Mode | Behavior |
| --- | --- |
| `cnpg` | Default. The chart creates a CloudNativePG `Cluster` resource from `database.cnpg.*`, and the API reads `DATABASE_URL` from the generated Secret named `<database.cnpg.name>-app`, key `uri`. |
| `external` | The chart does not create a database. `database.url` is required and is stored in the API Secret as `DATABASE_URL`, unless `existingSecret` is set. |

## Generated Credentials

### Valkey

When `valkey.enabled`, `valkey.auth.enabled`, and `valkey.auth.usersExistingSecret` are set, the parent chart creates the Valkey users Secret.

The default user's password is resolved in this order:

1. `valkey.auth.aclUsers.default.password`, when explicitly set.
2. Existing Kubernetes Secret data from `valkey.auth.usersExistingSecret`, when found by Helm `lookup`.
3. A newly generated password.

The same resolved password is used for the parent-managed users Secret and the API `REDIS_URL`.

### RustFS

When `objectStorage.mode=rustfs` and `rustfs.enabled=true`, the parent chart creates the RustFS credential Secret named by `objectStorage.rustfs.existingSecret`.

The RustFS access key and secret key are resolved in this order:

1. `objectStorage.rustfs.accessKey` and `objectStorage.rustfs.secretKey`, when explicitly set.
2. Existing Kubernetes Secret data from `objectStorage.rustfs.existingSecret`, when found by Helm `lookup`.
3. Newly generated credentials.

The same resolved credentials are used for the RustFS Secret and the API Secret's AWS credential keys.

`rustfs.secret.existingSecret` must match `objectStorage.rustfs.existingSecret`; the RustFS subchart consumes the parent-managed Secret through that value.

### Helm Template Caveat

Offline `helm template` runs cannot read live cluster Secrets, so generated credentials change between offline renders. Real `helm upgrade` operations against a cluster reuse existing generated credentials through Helm `lookup`.

## Common Examples

### Default CNPG Database With Bundled RustFS And Valkey

```bash
helm install autofile charts/autofile \
  --set secrets.JWT_SECRET='replace-me'
```

### External Database

```yaml
database:
  mode: external
  url: postgres://user:password@postgres.example.com:5432/autofile
```

### External S3-Compatible Storage

```yaml
objectStorage:
  mode: external
  bucket: autofile-documents

rustfs:
  enabled: false

secrets:
  AWS_ACCESS_KEY_ID: external-access-key
  AWS_SECRET_ACCESS_KEY: external-secret-key

api:
  extraEnv:
    - name: AWS_ENDPOINT_URL_S3
      value: https://s3.example.com
```

### AWS S3 With Ambient IAM

Use this when pods receive AWS credentials from the runtime environment, such as IRSA or another workload identity mechanism.

```yaml
objectStorage:
  mode: s3
  bucket: autofile-documents

rustfs:
  enabled: false

secrets:
  AWS_ACCESS_KEY_ID: ""
  AWS_SECRET_ACCESS_KEY: ""
```

### Explicit RustFS Credentials

```yaml
objectStorage:
  mode: rustfs
  rustfs:
    accessKey: my-rustfs-access-key
    secretKey: my-rustfs-secret-key
```

### Explicit Valkey Password

```yaml
valkey:
  auth:
    aclUsers:
      default:
        password: my-valkey-password
```

### Add API Environment Variables

`api.extraEnv` is appended to the API container's environment as standard Kubernetes env entries.

```yaml
api:
  extraEnv:
    - name: SOME_FEATURE_FLAG
      value: "true"
    - name: VALUE_FROM_SECRET
      valueFrom:
        secretKeyRef:
          name: my-secret
          key: my-key
```

### Enable Ingress

The chart creates separate Ingress resources for the API and UI. The API is served at `/api`; the UI is served at `/`.

```yaml
ingress:
  enabled: true
  className: nginx
  hosts:
    - host: autofile.example.com
  tls:
    - secretName: autofile-tls
      hosts:
        - autofile.example.com
```

## Parameters

### Global

| Parameter | Default | Description |
| --- | --- | --- |
| `nameOverride` | `""` | Override the chart name used in resource names. |
| `fullnameOverride` | `""` | Override the full release-derived resource name. |
| `imagePullSecrets` | `[]` | Image pull secrets added to API and UI pods. |

### Secrets

| Parameter | Default | Description |
| --- | --- | --- |
| `existingSecret` | `""` | Existing Secret containing API environment secrets. When set, `api.secret.yaml` is not rendered. For `database.mode=external`, this Secret must contain `DATABASE_URL`. |
| `secrets.REDIS_URL` | `redis://:changeme@valkey:6379/0` | Redis URL fallback when bundled authenticated Valkey is not active. |
| `secrets.JWT_SECRET` | `changeme` | JWT signing secret. Replace for any real deployment. |
| `secrets.AWS_ACCESS_KEY_ID` | `autofile-dev-access` | AWS access key for `external` mode, or for `s3` mode when both AWS credential values are non-empty. |
| `secrets.AWS_SECRET_ACCESS_KEY` | `autofile-dev-secret` | AWS secret key for `external` mode, or for `s3` mode when both AWS credential values are non-empty. |

### Database

| Parameter | Default | Description |
| --- | --- | --- |
| `database.mode` | `cnpg` | Database wiring mode: `cnpg` or `external`. |
| `database.url` | `""` | PostgreSQL connection URL required when `database.mode=external`. Rendered into the API Secret as `DATABASE_URL` when `existingSecret` is not set. |
| `database.cnpg.name` | `autofile-postgresql` | CloudNativePG `Cluster` resource name. The API reads CNPG's generated URI from Secret `<database.cnpg.name>-app`, key `uri`. |
| `database.cnpg.annotations` | `{}` | Annotations added to the CloudNativePG `Cluster`. |
| `database.cnpg.labels` | `{}` | Additional labels added to the CloudNativePG `Cluster`. |
| `database.cnpg.spec` | PostgreSQL 18, 1 instance, 5Gi storage | CloudNativePG `Cluster` spec rendered as-is. |

### Object Storage

| Parameter | Default | Description |
| --- | --- | --- |
| `objectStorage.mode` | `rustfs` | Object storage wiring mode: `rustfs`, `external`, or `s3`. |
| `objectStorage.bucket` | `autofile-documents` | Bucket used by the API for document files. Rendered as `S3_BUCKET`. |
| `objectStorage.rustfs.existingSecret` | `autofile-rustfs-secret` | Parent-managed Secret name for RustFS credentials. Must match `rustfs.secret.existingSecret`. |
| `objectStorage.rustfs.accessKey` | `""` | Optional explicit RustFS access key. Generated when empty. |
| `objectStorage.rustfs.secretKey` | `""` | Optional explicit RustFS secret key. Generated when empty. |
| `objectStorage.initContainer.image.repository` | `amazon/aws-cli` | Image repository for the API bucket-creation init container. |
| `objectStorage.initContainer.image.tag` | `latest` | Image tag for the API bucket-creation init container. |
| `objectStorage.initContainer.image.pullPolicy` | `IfNotPresent` | Pull policy for the API bucket-creation init container. |

### API

| Parameter | Default | Description |
| --- | --- | --- |
| `api.replicaCount` | `1` | Number of API replicas. |
| `api.image.repository` | `ghcr.io/joekhoobyar/autofile-api` | API image repository. |
| `api.image.pullPolicy` | `IfNotPresent` | API image pull policy. |
| `api.service.type` | `ClusterIP` | API Service type. |
| `api.service.port` | `80` | API Service port. |
| `api.service.annotations` | `{}` | API Service annotations. |
| `api.env.APP_MODE` | `development` | Application mode. |
| `api.env.BIND_ADDR` | `0.0.0.0:8000` | API bind address inside the container. |
| `api.env.RUST_LOG` | `info` | Rust tracing filter. |
| `api.env.AWS_REGION` | `us-east-1` | AWS/S3 region. |
| `api.env.ALLOWED_ORIGINS` | `""` | Optional comma-separated CORS origins. Omitted when empty. |
| `api.extraEnv` | `[]` | Additional Kubernetes env entries appended to the API container. |
| `api.startupProbe.*` | see `values.yaml` | API startup probe timing values. |
| `api.livenessProbe.*` | see `values.yaml` | API liveness probe timing values. |
| `api.readinessProbe.*` | see `values.yaml` | API readiness probe timing values. |
| `api.resources` | `{}` | API container resource requests and limits. |

### UI

| Parameter | Default | Description |
| --- | --- | --- |
| `ui.replicaCount` | `1` | Number of UI replicas. |
| `ui.image.repository` | `ghcr.io/joekhoobyar/autofile-ui` | UI image repository. |
| `ui.image.pullPolicy` | `IfNotPresent` | UI image pull policy. |
| `ui.service.type` | `ClusterIP` | UI Service type. |
| `ui.service.port` | `80` | UI Service port. |
| `ui.service.annotations` | `{}` | UI Service annotations. |
| `ui.livenessProbe.*` | see `values.yaml` | UI liveness probe timing values. |
| `ui.readinessProbe.*` | see `values.yaml` | UI readiness probe timing values. |
| `ui.resources` | `{}` | UI container resource requests and limits. |

### Service Account, Security, And Scheduling

| Parameter | Default | Description |
| --- | --- | --- |
| `serviceAccount.create` | `true` | Create a ServiceAccount for the API. |
| `serviceAccount.annotations` | `{}` | ServiceAccount annotations. |
| `serviceAccount.name` | `null` | Existing or explicit ServiceAccount name. Defaults to the chart fullname when creating. |
| `podSecurityContext` | `{ fsGroup: 1000 }` | Pod-level security context for API and UI pods. |
| `securityContext` | drops all capabilities, non-root | Container-level security context for API and UI containers. |
| `nodeSelector` | `{}` | Node selector applied to API and UI pods. |
| `tolerations` | `[]` | Tolerations applied to API and UI pods. |
| `affinity` | `{}` | Affinity applied to API and UI pods. |

### Ingress

| Parameter | Default | Description |
| --- | --- | --- |
| `ingress.enabled` | `false` | Create API and UI Ingress resources. |
| `ingress.className` | `""` | IngressClass name. |
| `ingress.annotations` | `{}` | Annotations applied to both API and UI Ingress resources. |
| `ingress.hosts` | `[{ host: chart-example.local }]` | Hosts used by both API and UI Ingress resources. |
| `ingress.tls` | `[]` | TLS configuration applied to both API and UI Ingress resources. |

### Valkey Dependency

| Parameter | Default | Description |
| --- | --- | --- |
| `valkey.enabled` | `true` | Install the Valkey dependency. |
| `valkey.auth.enabled` | `true` | Enable Valkey ACL authentication. |
| `valkey.auth.usersExistingSecret` | `{{ .Release.Name }}-valkey-users` | Parent-managed Valkey users Secret name. Supports templating. |
| `valkey.auth.aclUsers.default.password` | `""` | Optional explicit default-user password. Generated when empty. |
| `valkey.auth.aclUsers.default.permissions` | `~* &* +@all` | ACL permissions for the default user. |

Additional `valkey.*` values are passed through to the Valkey subchart. See the Valkey chart documentation for the full dependency value surface.

### RustFS Dependency

| Parameter | Default | Description |
| --- | --- | --- |
| `rustfs.enabled` | `true` | Install the RustFS dependency. |
| `rustfs.mode.standalone.enabled` | `true` | Deploy RustFS in standalone mode by default. |
| `rustfs.mode.distributed.enabled` | `false` | Distributed RustFS mode. Disabled by default for this application chart. |
| `rustfs.ingress.enabled` | `false` | RustFS subchart ingress. Disabled by default. |
| `rustfs.secret.existingSecret` | `autofile-rustfs-secret` | RustFS credential Secret consumed by the subchart. Must match `objectStorage.rustfs.existingSecret`. |

Additional `rustfs.*` values are passed through to the RustFS subchart. See the RustFS chart documentation for the full dependency value surface.

## Keeping This README Updated

When changing chart values or template behavior, update this README in the same change. At minimum, update:

- The matching parameter table row.
- Any affected object storage or generated credential behavior.
- Any examples that use renamed or removed values.
- The validation commands if chart testing expectations change.
