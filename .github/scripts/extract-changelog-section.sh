#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 VERSION CHANGELOG [--output FILE]" >&2
}

version="${1:-}"
changelog="${2:-}"
output=""

if [ -z "${version}" ] || [ -z "${changelog}" ]; then
  usage
  exit 2
fi

shift 2
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      output="${2:-}"
      if [ -z "${output}" ]; then
        usage
        exit 2
      fi
      shift 2
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

if [ ! -f "${changelog}" ]; then
  echo "Changelog not found: ${changelog}" >&2
  exit 1
fi

if ! [[ "${version}" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "Version must be semver, optionally prefixed with v: ${version}" >&2
  exit 1
fi

normalized_version="${version#v}"
tmp_file="$(mktemp)"
trap 'rm -f "${tmp_file}"' EXIT

awk -v version="${normalized_version}" '
  function regex_escape(value, escaped) {
    escaped = value
    gsub(/[][\\.^$*+?(){}|]/, "\\\\&", escaped)
    return escaped
  }

  function trim_trailing_blank_lines() {
    while (count > 0 && lines[count] ~ /^[[:space:]]*$/) {
      count--
    }
  }

  BEGIN {
    in_section = 0
    found = 0
    count = 0
    version_pattern = "^##[[:space:]]+\\[?v?" regex_escape(version) "\\]?([[:space:]]|$|-)"
  }

  $0 ~ /^##[[:space:]]+/ {
    if (in_section) {
      exit
    }
    if ($0 ~ version_pattern) {
      in_section = 1
      found = 1
      next
    }
  }

  in_section {
    if (count == 0 && $0 ~ /^[[:space:]]*$/) {
      next
    }
    lines[++count] = $0
  }

  END {
    if (!found) {
      exit 3
    }
    trim_trailing_blank_lines()
    if (count == 0) {
      exit 4
    }
    for (i = 1; i <= count; i++) {
      print lines[i]
    }
  }
' "${changelog}" > "${tmp_file}" || status="$?"

status="${status:-0}"
case "${status}" in
  0)
    ;;
  3)
    echo "No CHANGELOG.md section found for version ${normalized_version}." >&2
    echo "Available version headings:" >&2
    grep -E '^##[[:space:]]+' "${changelog}" >&2 || true
    exit 1
    ;;
  4)
    echo "CHANGELOG.md section for version ${normalized_version} is empty." >&2
    exit 1
    ;;
  *)
    exit "${status}"
    ;;
esac

if [ -n "${output}" ]; then
  cp "${tmp_file}" "${output}"
else
  cat "${tmp_file}"
fi
