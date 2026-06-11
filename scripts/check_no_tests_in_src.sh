#!/usr/bin/env bash
set -euo pipefail

mode="${TAK_LINE_MODE:-working-tree}"
base_ref="${TAK_BASE_REF:-origin/main}"

script_root="$(cd "$(dirname "$0")/.." && pwd)"
repo_root="$(git -C "$script_root" rev-parse --show-toplevel 2>/dev/null || printf '%s\n' "$script_root")"
cd "$repo_root"

has_git_worktree() {
  git rev-parse --is-inside-work-tree >/dev/null 2>&1
}

list_src_files_without_git() {
  [[ -d crates ]] || return 0
  find crates -type f -name '*.rs' | rg '/src/' || true
}

list_all_src_files() {
  if ! has_git_worktree; then
    list_src_files_without_git
    return
  fi
  git ls-files 'crates/**/*.rs' | rg '/src/'
}

list_base_ref_src_files() {
  if ! has_git_worktree; then
    return
  fi
  if git rev-parse --verify "$base_ref" >/dev/null 2>&1; then
    local merge_base
    merge_base="$(git merge-base HEAD "$base_ref")"
    git diff --name-only --diff-filter=ACMR "$merge_base"...HEAD -- 'crates/**/*.rs' | rg '/src/' || true
  else
    git diff --name-only --diff-filter=ACMR HEAD -- 'crates/**/*.rs' | rg '/src/' || true
  fi
}

list_working_tree_src_files() {
  if ! has_git_worktree; then
    return
  fi
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
    if is_doc_only_changed_file "$file"; then
      continue
    fi
    echo "src-test-separation-check: ${file} contains test attributes"
    echo "$violations"
    failures=1
  fi
done <<< "$candidates"

if (( failures != 0 )); then
  exit 1
fi

echo "src-test-separation-check: ok (${mode} mode)."
