#!/usr/bin/env bash
set -euo pipefail

src_limit="${TAK_SRC_LINE_LIMIT:-200}"
test_limit="${TAK_TEST_LINE_LIMIT:-100}"
mode="${TAK_LINE_MODE:-working-tree}"
base_ref="${TAK_BASE_REF:-origin/main}"

script_root="$(cd "$(dirname "$0")/.." && pwd)"
repo_root="$(git -C "$script_root" rev-parse --show-toplevel 2>/dev/null || printf '%s\n' "$script_root")"
cd "$repo_root"

has_git_worktree() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1
}

list_files_without_git() {
  [[ -d crates ]] || return 0
  find crates -type f -name '*.rs' | sort
}

list_all_files() {
  if ! has_git_worktree; then
    list_files_without_git
    return
  fi
  git ls-files 'crates/**/*.rs'
}

list_base_ref_files() {
  if ! has_git_worktree; then
    return
  fi
  if git rev-parse --verify "$base_ref" >/dev/null 2>&1; then
    local merge_base
    merge_base="$(git merge-base HEAD "$base_ref")"
    git diff --name-only --diff-filter=ACMR "$merge_base"...HEAD -- 'crates/**/*.rs'
  else
    git diff --name-only --diff-filter=ACMR HEAD -- 'crates/**/*.rs'
  fi
}

list_working_tree_files() {
  if ! has_git_worktree; then
    return
  fi
  git diff --name-only --diff-filter=ACMR HEAD -- 'crates/**/*.rs'
  git ls-files --others --exclude-standard -- 'crates/**/*.rs'
}

is_test_file() {
  case "$1" in
    */tests/*) return 0 ;;
    */src/*_tests.rs) return 0 ;;
    */src/*_test.rs) return 0 ;;
    */src/lib_tests.rs) return 0 ;;
    *) return 1 ;;
  esac
}

is_doc_only_changed_file() {
  local file="$1"

  [[ "$mode" == "working-tree" ]] || return 1
  has_git_worktree || return 1
  git diff --quiet HEAD -- "$file" && return 1

  local changed=0
  local line
  while IFS= read -r line; do
    case "$line" in
      diff\ --git*|index\ *|---\ *|+++\ *|@@\ *) continue ;;
      +*|-*)
        changed=1
        local content="${line:1}"
        if [[ ! "$content" =~ ^[[:space:]]*$ && ! "$content" =~ ^[[:space:]]*/// ]]; then
          return 1
        fi
        ;;
    esac
  done < <(git diff --unified=0 HEAD -- "$file")

  (( changed != 0 ))
}

case "$mode" in
  all)
    candidates="$(list_all_files | sort -u)"
    ;;
  base-ref)
    candidates="$(list_base_ref_files | sort -u)"
    ;;
  working-tree)
    candidates="$(list_working_tree_files | sort -u)"
    ;;
  *)
    echo "line-limit-check: unsupported TAK_LINE_MODE=${mode}; use all|base-ref|working-tree" >&2
    exit 2
    ;;
esac

if [[ -z "$candidates" ]]; then
  echo "line-limit-check: no matching Rust files to validate (${mode} mode)."
  exit 0
fi

failures=0
while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  [[ -f "$file" ]] || continue

  line_count="$(wc -l < "$file" | tr -d ' ')"
  limit="$src_limit"
  if is_test_file "$file"; then
    limit="$test_limit"
  fi

  if (( line_count > limit )); then
    if is_doc_only_changed_file "$file"; then
      continue
    fi
    echo "line-limit-check: ${file}:${line_count} exceeds limit ${limit}"
    failures=1
  fi
done <<< "$candidates"

if (( failures != 0 )); then
  exit 1
fi

echo "line-limit-check: ok (${mode} mode)."
