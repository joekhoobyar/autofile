{{- define "autofile.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "autofile.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "autofile.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "autofile.api.imageTag" -}}
{{- .Values.api.image.tag | default .Chart.AppVersion -}}
{{- end -}}

{{- define "autofile.ui.imageTag" -}}
{{- .Values.ui.image.tag | default .Chart.AppVersion -}}
{{- end -}}

{{- define "autofile.labels" -}}
helm.sh/chart: {{ include "autofile.chart" . }}
{{ include "autofile.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{- define "autofile.selectorLabels" -}}
app.kubernetes.io/name: {{ include "autofile.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "autofile.api.selectorLabels" -}}
{{ include "autofile.selectorLabels" . }}
app.kubernetes.io/component: api
{{- end -}}

{{- define "autofile.ui.selectorLabels" -}}
{{ include "autofile.selectorLabels" . }}
app.kubernetes.io/component: ui
{{- end -}}

{{- define "autofile.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{ default (include "autofile.fullname" .) .Values.serviceAccount.name }}
{{- else -}}
{{ default "default" .Values.serviceAccount.name }}
{{- end -}}
{{- end -}}

{{- define "autofile.api.fullname" -}}
{{ include "autofile.fullname" . }}-api
{{- end -}}

{{- define "autofile.ui.fullname" -}}
{{ include "autofile.fullname" . }}-ui
{{- end -}}

{{- define "autofile.secretName" -}}
{{ .Values.existingSecret | default (printf "%s-env" (include "autofile.fullname" .)) }}
{{- end -}}

{{- define "autofile.valkey.enabledWithAuth" -}}
{{- if and .Values.valkey.enabled .Values.valkey.auth.enabled .Values.valkey.auth.usersExistingSecret -}}
true
{{- end -}}
{{- end -}}

{{- define "autofile.valkey.fullname" -}}
{{- if .Values.valkey.fullnameOverride -}}
{{- .Values.valkey.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default "valkey" .Values.valkey.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "autofile.valkey.usersSecretName" -}}
{{- tpl .Values.valkey.auth.usersExistingSecret . -}}
{{- end -}}

{{- define "autofile.valkey.defaultPasswordKey" -}}
{{- $defaultUser := index .Values.valkey.auth.aclUsers "default" | default dict -}}
{{- $defaultUser.passwordKey | default "default" -}}
{{- end -}}

{{- define "autofile.valkey.defaultPassword" -}}
{{- if .Values._autofileValkeyDefaultPassword -}}
{{- .Values._autofileValkeyDefaultPassword -}}
{{- else -}}
{{- $secretName := include "autofile.valkey.usersSecretName" . -}}
{{- $passwordKey := include "autofile.valkey.defaultPasswordKey" . -}}
{{- $password := randAlphaNum 32 -}}
{{- $existingSecret := lookup "v1" "Secret" .Release.Namespace $secretName -}}
{{- if and $existingSecret $existingSecret.data (hasKey $existingSecret.data $passwordKey) -}}
{{- $password = (index $existingSecret.data $passwordKey | b64dec) -}}
{{- end -}}
{{- $_ := set .Values "_autofileValkeyDefaultPassword" $password -}}
{{- $password -}}
{{- end -}}
{{- end -}}

{{- define "autofile.redisURL" -}}
{{- if include "autofile.valkey.enabledWithAuth" . -}}
{{- $password := include "autofile.valkey.defaultPassword" . | urlquery -}}
{{- printf "redis://default:%s@%s:6379/0" $password (include "autofile.valkey.fullname" .) -}}
{{- else -}}
{{- .Values.secrets.REDIS_URL -}}
{{- end -}}
{{- end -}}

{{- define "autofile.objectStorage.isRustFS" -}}
{{- if and .Values.rustfs.enabled (eq .Values.objectStorage.mode "rustfs") -}}
true
{{- end -}}
{{- end -}}

{{- define "autofile.rustfs.fullname" -}}
{{- if .Values.rustfs.fullnameOverride -}}
{{- .Values.rustfs.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default "rustfs" .Values.rustfs.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "autofile.rustfs.endpointURL" -}}
{{- printf "http://%s-svc:%v" (include "autofile.rustfs.fullname" .) (.Values.rustfs.service.endpoint.port | default 9000) -}}
{{- end -}}

{{- define "autofile.objectStorage.endpointURL" -}}
{{- if include "autofile.objectStorage.isRustFS" . -}}
{{- include "autofile.rustfs.endpointURL" . -}}
{{- end -}}
{{- end -}}

{{- define "autofile.rustfs.secretName" -}}
{{- .Values.objectStorage.rustfs.existingSecret -}}
{{- end -}}

{{- define "autofile.rustfs.accessKey" -}}
{{- if .Values.objectStorage.rustfs.accessKey -}}
{{- .Values.objectStorage.rustfs.accessKey -}}
{{- else -}}
{{- $secretName := include "autofile.rustfs.secretName" . -}}
{{- $accessKey := randAlphaNum 32 -}}
{{- $existingSecret := lookup "v1" "Secret" .Release.Namespace $secretName -}}
{{- if and $existingSecret $existingSecret.data (hasKey $existingSecret.data "RUSTFS_ACCESS_KEY") -}}
{{- $accessKey = (index $existingSecret.data "RUSTFS_ACCESS_KEY" | b64dec) -}}
{{- end -}}
{{- $_ := set .Values.objectStorage.rustfs "accessKey" $accessKey -}}
{{- $accessKey -}}
{{- end -}}
{{- end -}}

{{- define "autofile.rustfs.secretKey" -}}
{{- if .Values.objectStorage.rustfs.secretKey -}}
{{- .Values.objectStorage.rustfs.secretKey -}}
{{- else -}}
{{- $secretName := include "autofile.rustfs.secretName" . -}}
{{- $secretKey := randAlphaNum 64 -}}
{{- $existingSecret := lookup "v1" "Secret" .Release.Namespace $secretName -}}
{{- if and $existingSecret $existingSecret.data (hasKey $existingSecret.data "RUSTFS_SECRET_KEY") -}}
{{- $secretKey = (index $existingSecret.data "RUSTFS_SECRET_KEY" | b64dec) -}}
{{- end -}}
{{- $_ := set .Values.objectStorage.rustfs "secretKey" $secretKey -}}
{{- $secretKey -}}
{{- end -}}
{{- end -}}

{{- define "autofile.awsAccessKeyID" -}}
{{- if include "autofile.objectStorage.isRustFS" . -}}
{{- include "autofile.rustfs.accessKey" . -}}
{{- else -}}
{{- .Values.secrets.AWS_ACCESS_KEY_ID -}}
{{- end -}}
{{- end -}}

{{- define "autofile.awsSecretAccessKey" -}}
{{- if include "autofile.objectStorage.isRustFS" . -}}
{{- include "autofile.rustfs.secretKey" . -}}
{{- else -}}
{{- .Values.secrets.AWS_SECRET_ACCESS_KEY -}}
{{- end -}}
{{- end -}}

{{- define "autofile.awsCredentialsEnabled" -}}
{{- if include "autofile.objectStorage.isRustFS" . -}}
true
{{- else if eq .Values.objectStorage.mode "external" -}}
true
{{- else if and (eq .Values.objectStorage.mode "s3") .Values.secrets.AWS_ACCESS_KEY_ID .Values.secrets.AWS_SECRET_ACCESS_KEY -}}
true
{{- end -}}
{{- end -}}
