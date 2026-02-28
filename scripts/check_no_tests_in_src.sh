#!/usr/bin/env bash
set -euo pipefail

mode="${TAK_LINE_MODE:-working-tree}"
base_ref="${TAK_BASE_REF:-origin/main}"

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

list_all_src_files() {
  git ls-files 'crates/**/*.rs' | rg '/src/'
}

list_base_ref_src_files() {
  if git rev-parse --verify "$base_ref" >/dev/null 2>&1; then
    local merge_base
    merge_base="$(git merge-base HEAD "$base_ref")"
    git diff --name-only --diff-filter=ACMR "$merge_base"...HEAD -- 'crates/**/*.rs' | rg '/src/' || true
  else
    git diff --name-only --diff-filter=ACMR HEAD -- 'crates/**/*.rs' | rg '/src/' || true
  fi
}

list_working_tree_src_files() {
  git diff --name-only --diff-filter=ACMR HEAD -- 'crates/**/*.rs' | rg '/src/' || true
  git ls-files --others --exclude-standard -- 'crates/**/*.rs' | rg '/src/' || true
}

is_test_source_file() {
  case "$1" in
    */src/*_tests.rs) return 0 ;;
    */src/*_test.rs) return 0 ;;
    */src/lib_tests.rs) return 0 ;;
    */src/*/tests.rs) return 0 ;;
    */src/*/tests/*.rs) return 0 ;;
    *) return 1 ;;
  esac
}

disallowed_test_markers() {
  local file="$1"
  awk '
    /#\[test\]/ {
      print NR ":" $0
      bad = 1
      next
    }

    /#\[cfg\(test\)\]/ {
      cfg_nr = NR
      cfg_line = $0
      pending_cfg = 1
      next
    }

    pending_cfg {
      if ($0 ~ /^[[:space:]]*$/) {
        next
      }
      if ($0 ~ /^[[:space:]]*mod[[:space:]]+[A-Za-z0-9_]+;[[:space:]]*$/) {
        pending_cfg = 0
        next
      }

      print cfg_nr ":" cfg_line
      bad = 1
      pending_cfg = 0
    }

    END {
      if (pending_cfg) {
        print cfg_nr ":" cfg_line
        bad = 1
      }
      if (!bad) {
        exit 1
      }
    }
  ' "$file" || true
}

case "$mode" in
  all)
    candidates="$(list_all_src_files | sort -u)"
    ;;
  base-ref)
    candidates="$(list_base_ref_src_files | sort -u)"
    ;;
  working-tree)
    candidates="$(list_working_tree_src_files | sort -u)"
    ;;
  *)
    echo "src-test-separation-check: unsupported TAK_LINE_MODE=${mode}; use all|base-ref|working-tree" >&2
    exit 2
    ;;
esac

if [[ -z "$candidates" ]]; then
  echo "src-test-separation-check: no matching source files to validate (${mode} mode)."
  exit 0
fi

failures=0
while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  [[ -f "$file" ]] || continue
  if is_test_source_file "$file"; then
    continue
  fi

  violations="$(disallowed_test_markers "$file")"
  if [[ -n "$violations" ]]; then
    echo "src-test-separation-check: ${file} contains test attributes"
    echo "$violations"
    failures=1
  fi
done <<< "$candidates"

if (( failures != 0 )); then
  exit 1
fi

echo "src-test-separation-check: ok (${mode} mode)."
