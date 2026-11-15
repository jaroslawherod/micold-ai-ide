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
FR-003: only `state.mode=file` exists in this chart. The other names are reserved for the issues
that will implement them, and rendering them fails rather than deploying something that ignores
the setting.
*/}}
{{- define "micold.stateMode" -}}
{{- if ne .Values.state.mode "file" -}}
{{- fail (printf "state.mode=%s is not available yet: only state.mode=file is supported; sqlite arrives with #1948 and postgres with #1740" .Values.state.mode) -}}
{{- end -}}
{{- end -}}

{{/* The pod security context every component uses (US1-1). Not a value: it cannot be weakened. */}}
{{- define "micold.podSecurityContext" -}}
runAsNonRoot: true
runAsUser: 65532
runAsGroup: 65532
fsGroup: 65532
seccompProfile:
  type: RuntimeDefault
{{- end -}}

{{- define "micold.containerSecurityContext" -}}
runAsNonRoot: true
runAsUser: 65532
runAsGroup: 65532
allowPrivilegeEscalation: false
readOnlyRootFilesystem: true
capabilities:
  drop:
    - ALL
{{- end -}}
