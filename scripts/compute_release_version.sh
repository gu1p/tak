#!/usr/bin/env bash
set -euo pipefail

workspace_version="${1:?workspace version is required}"
head_sha="${2:?head sha is required}"

case "$workspace_version" in
  *.*.*) ;;
  *)
    echo "compute_release_version: workspace version must be MAJOR.MINOR.PATCH" >&2
    exit 1
    ;;
esac

major="${workspace_version%%.*}"
minor_patch="${workspace_version#*.}"
minor="${minor_patch%%.*}"
workspace_patch="${minor_patch#*.}"

case "$major:$minor:$workspace_patch" in
  *[!0-9:]* | :: | *::*)
    echo "compute_release_version: workspace version must be numeric SemVer" >&2
    exit 1
    ;;
esac

max_patch="$workspace_patch"
head_patch=""

while IFS= read -r tag; do
  patch="${tag#v${major}.${minor}.}"
  case "$patch" in
    '' | *[!0-9]*) continue ;;
  esac

  if [ "$patch" -gt "$max_patch" ]; then
    max_patch="$patch"
  fi

  tag_sha="$(git rev-list -n1 "$tag")"
  if [ "$tag_sha" = "$head_sha" ]; then
    if [ -z "$head_patch" ] || [ "$patch" -gt "$head_patch" ]; then
      head_patch="$patch"
    fi
  fi
done < <(git tag --list "v${major}.${minor}.*")

if [ -n "$head_patch" ]; then
  next_patch="$head_patch"
else
  next_patch=$((max_patch + 1))
fi

version="${major}.${minor}.${next_patch}"
printf 'tag=v%s\n' "$version"
printf 'version=%s\n' "$version"
