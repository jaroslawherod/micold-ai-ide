{{/*
The release's base name: the release name, cut to the 63 characters a Kubernetes name allows.
*/}}
{{- define "micold.fullname" -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "micold.labels" -}}
app.kubernetes.io/name: micold
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
{{- end -}}

{{/* Selector labels of one component (`orchestrator`, `web`); pass (list $ "<component>"). */}}
{{- define "micold.selectorLabels" -}}
{{- $root := index . 0 -}}
app.kubernetes.io/name: micold
app.kubernetes.io/instance: {{ $root.Release.Name }}
app.kubernetes.io/component: {{ index . 1 }}
{{- end -}}

{{/* image.repository:tag, the tag defaulting to the chart's appVersion. Pass (list $ .Values.image). */}}
{{- define "micold.image" -}}
{{- $root := index . 0 -}}
{{- $image := index . 1 -}}
{{- printf "%s:%s" $image.repository (default $root.Chart.AppVersion $image.tag) -}}
{{- end -}}

{{/*
FR-003, FR-020 of #1732: the chart runs `state.mode=file` (a JSON registry on the data volume) and
`state.mode=postgres` (the #1948 store, with `postgres.enabled` or `database.existingSecret`).
Anything else, `sqlite` included (the store's local-only mode), fails rather than deploying
something that ignores the setting.
*/}}
{{- define "micold.stateMode" -}}
{{- $mode := .Values.state.mode -}}
{{- if not (has $mode (list "file" "postgres")) -}}
{{- fail (printf "state.mode=%s is not supported by this chart: use file or postgres" $mode) -}}
{{- end -}}
{{- if and (eq $mode "postgres") (not .Values.postgres.enabled) (not .Values.database.existingSecret) -}}
{{- fail "state.mode=postgres needs postgres.enabled=true or database.existingSecret" -}}
{{- end -}}
{{- end -}}

{{/* The Secret whose `url` key holds the database URL: the given one, else the chart's Postgres. */}}
{{- define "micold.databaseSecret" -}}
{{- default (printf "%s-postgres" (include "micold.fullname" .)) .Values.database.existingSecret -}}
{{- end -}}

{{/* Whether the data volume is a PVC: persistence on, and not the seeded `volume` breakage. */}}
{{- define "micold.persistent" -}}
{{- if and .Values.state.persistence.enabled (ne .Values.testing.breakage "volume") -}}true{{- end -}}
{{- end -}}

{{/*
The pod security context every component uses (US1-1). Not a value: it cannot be weakened, except
by the test-only seeded breakage `testing.breakage=user` (root), which the e2e suite must catch.
*/}}
{{- define "micold.podSecurityContext" -}}
{{- if eq .Values.testing.breakage "user" -}}
runAsNonRoot: false
runAsUser: 0
runAsGroup: 0
{{- else -}}
runAsNonRoot: true
runAsUser: 65532
runAsGroup: 65532
{{- end }}
fsGroup: 65532
seccompProfile:
  type: RuntimeDefault
{{- end -}}

{{- define "micold.containerSecurityContext" -}}
{{- if eq .Values.testing.breakage "user" -}}
runAsNonRoot: false
runAsUser: 0
runAsGroup: 0
{{- else -}}
runAsNonRoot: true
runAsUser: 65532
runAsGroup: 65532
{{- end }}
allowPrivilegeEscalation: false
readOnlyRootFilesystem: true
capabilities:
  drop:
    - ALL
{{- end -}}
