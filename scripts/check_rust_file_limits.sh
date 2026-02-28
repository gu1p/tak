#!/usr/bin/env bash
set -euo pipefail

src_limit="${TAK_SRC_LINE_LIMIT:-200}"
test_limit="${TAK_TEST_LINE_LIMIT:-100}"
mode="${TAK_LINE_MODE:-working-tree}"
base_ref="${TAK_BASE_REF:-origin/main}"

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

list_all_files() {
  git ls-files 'crates/**/*.rs'
}

list_base_ref_files() {
  if git rev-parse --verify "$base_ref" >/dev/null 2>&1; then
    local merge_base
    merge_base="$(git merge-base HEAD "$base_ref")"
    git diff --name-only --diff-filter=ACMR "$merge_base"...HEAD -- 'crates/**/*.rs'
  else
    git diff --name-only --diff-filter=ACMR HEAD -- 'crates/**/*.rs'
  fi
}

list_working_tree_files() {
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
    echo "line-limit-check: ${file}:${line_count} exceeds limit ${limit}"
    failures=1
  fi
done <<< "$candidates"

if (( failures != 0 )); then
  exit 1
fi

echo "line-limit-check: ok (${mode} mode)."
