#!/usr/bin/env bash
set -euo pipefail

repo="${GITHUB_REPOSITORY:-}"
confirm="false"

usage() {
  printf 'Usage: %s [--repo OWNER/REPO] [--yes]\n' "${0##*/}"
  printf '\n'
  printf 'Deletes GitHub Code Scanning analyses whose category starts with trivy-.\n'
  printf 'Runs in dry-run mode unless --yes is provided. Requires gh authentication.\n'
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo)
      if [ -z "${2:-}" ]; then
        printf '%s\n\n' '--repo requires OWNER/REPO.' >&2
        usage >&2
        exit 2
      fi
      repo="${2:-}"
      shift 2
      ;;
    --yes)
      confirm="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'Unknown argument: %s\n\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ -z "${repo}" ]; then
  if ! command -v gh >/dev/null 2>&1; then
    printf 'gh is required.\n' >&2
    exit 1
  fi
  repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
elif ! command -v gh >/dev/null 2>&1; then
  printf 'gh is required.\n' >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  printf 'jq is required.\n' >&2
  exit 1
fi

analyses="$(gh api --paginate "/repos/${repo}/code-scanning/analyses?per_page=100" \
  | jq -r '.[] | select((.category // "") | startswith("trivy-")) | [.id, .category, .ref] | @tsv')"

if [ -z "${analyses}" ]; then
  printf 'No trivy-* code scanning analyses found for %s.\n' "${repo}"
  exit 0
fi

if [ "${confirm}" != "true" ]; then
  printf 'Dry run: would delete these code scanning analyses from %s:\n' "${repo}"
  printf '%s\n' "${analyses}"
  printf '\nRun again with --yes to delete them.\n'
  exit 0
fi

while IFS=$'\t' read -r analysis_id category ref; do
  printf 'Deleting analysis %s (%s, %s)\n' "${analysis_id}" "${category}" "${ref}"
  gh api --method DELETE "/repos/${repo}/code-scanning/analyses/${analysis_id}?confirm_delete"
done <<< "${analyses}"
