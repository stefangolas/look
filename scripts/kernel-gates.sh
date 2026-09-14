#!/usr/bin/env bash
# P-3 kernel gates. Enforce H-1/H-3/H-4 on NEW kernel code only, where "kernel
# code" means the vendored truck tree that the B-rep generation kernel (BG-*)
# edits: vendor/truck/**.
#
# Every gate is diff-based against a baseline ref, so pre-existing violations
# in the baseline are never flagged.
#
# Self-removing vendoring exclusion:
#   While the baseline ref does NOT contain vendor/truck/ (the P-1 vendoring
#   commit has not yet landed there), the whole vendored tree is an upstream
#   snapshot being moved in, not new kernel code, so the gates are no-ops.
#   Once the baseline contains vendor/truck/, the exclusion disappears and
#   kernel edits to the tree are gated normally.
#
# Usage: kernel-gates.sh <baseline-ref>
#   baseline-ref: commit the diff is computed against (merge-base for PRs,
#                 the pushed-to HEAD's parent for push builds).
#
# Exit code 0 = gates pass; 1 = a gate failed; 2 = usage error.
set -euo pipefail

base="${1:-}"
if [ -z "$base" ] || ! git rev-parse --verify "$base" >/dev/null 2>&1; then
    echo "usage: kernel-gates.sh <baseline-ref>" >&2
    exit 2
fi

# The kernel tree is vendor/truck/. Only gateable once the baseline contains it
# (i.e. the vendoring move has landed); before that the tree is an upstream
# snapshot being moved in, not new kernel code, and the gates are no-ops.
if ! git cat-file -e "$base:vendor/truck" 2>/dev/null; then
    echo "kernel-gates: baseline does not contain vendor/truck/ (vendoring not landed); gates are no-ops."
    exit 0
fi

# Added lines in kernel *.rs files relative to the baseline. Filters out the
# '+++ b/path' headers git emits.
added_rs_lines() {
    git diff --unified=0 "$base"...HEAD -- 'vendor/truck' -- '*.rs' \
        | grep '^+' | grep -v '^+++' || true
}

# Kernel .rs files newly added relative to the baseline (not present in base
# tree).
new_rs_files() {
    # NOTE (session 42): the previous pathspec form
    # `git diff -- vendor/truck -- '*.rs'` does NOT filter — the second
    # `--` is not a second separator, so every new vendor file (including
    # data files like proptest-regressions/*.txt) leaked into GATE-1 and
    # misfired on non-Rust content. Filter the extension explicitly.
    git diff --name-status -M "$base"...HEAD -- 'vendor/truck' \
        | awk -F'\t' '
            $1 ~ /^A/ && $2 ~ /\.rs$/ { print $2 }
            $1 ~ /^R/ && $3 ~ /\.rs$/ { print $3 "\t" $2 }
          ' \
        | while IFS= read -r entry; do
            case "$entry" in
                *$'\t'*)
                    # Rename destination (session 46): absent at base by
                    # definition, but a rename whose SOURCE existed at base
                    # is relocated baseline code, not new code — GATE-1's own
                    # header disclaims pre-existing baseline content, and
                    # BG-CK-P0-CRATE's 53 moved modules (252 grandfathered
                    # unwraps) must not be force-fitted with an H-1 deny
                    # their source never carried.
                    f=${entry%%$'\t'*}; src=${entry#*$'\t'}
                    if git cat-file -e "$base:$src" 2>/dev/null; then
                        continue
                    fi
                    printf '%s\n' "$f"
                    ;;
                *)
                    f=$entry
                    if ! git cat-file -e "$base:$f" 2>/dev/null; then
                        printf '%s\n' "$f"
                    fi
                    ;;
            esac
        done
}

fail=0

# ---------------------------------------------------------------------------
# Gate 1 (P-3): every NEW kernel module declares the H-1 lint denials.
# The deny attribute is the per-module contract; clippy enforces it once
# declared. `#[allow]` must carry an inline justification comment on the same
# line or immediately above it.
# ---------------------------------------------------------------------------
if new=$(new_rs_files); then
    if [ -n "$new" ]; then
        while IFS= read -r f; do
            if ! grep -q 'clippy::unwrap_used' "$f"; then
                echo "GATE-1: new kernel module $f does not deny clippy::unwrap_used (H-1)." >&2
                fail=1
            fi
        done <<<"$new"
    fi
fi

# ---------------------------------------------------------------------------
# Gate 2 (H-3): no bare absolute length literals in added kernel predicate
# lines. Catches the S-2 class: a literal 1e-6-class tolerance compared
# against a model-space length. Dimensionless comparisons are permitted and
# must name the quantity in a comment; lines may opt out with a `// H-3`
# marker.
# ---------------------------------------------------------------------------
literals=$(added_rs_lines | grep -E '[^a-zA-Z0-9_]1(\.0+)?e-0?[0-9]+' || true)
if [ -n "$literals" ]; then
    # Drop lines that carry an explicit H-3 opt-out comment.
    allowed=$(printf '%s\n' "$literals" | grep '// H-3' || true)
    violations=$(comm -23 <(printf '%s\n' "$literals" | sort -u) <(printf '%s\n' "$allowed" | sort -u) || true)
    if [ -n "$violations" ]; then
        echo "GATE-2 (H-3): bare absolute length literal in added kernel code:" >&2
        printf '%s\n' "$violations" >&2
        echo "  Move the comparison through ToleranceCtx, or mark the line '// H-3'." >&2
        fail=1
    fi
fi

# ---------------------------------------------------------------------------
# Gate 3 (H-4): no debug_new and no cfg!(debug_assertions)-dependent semantics
# in added kernel lines. The audit (F-7) bans the debug-vs-release validity
# split.
# ---------------------------------------------------------------------------
debug_hits=$(added_rs_lines | grep -E '\bdebug_new\b|cfg!\(debug_assertions\)' || true)
if [ -n "$debug_hits" ]; then
    echo "GATE-3 (H-4): debug_new or cfg!(debug_assertions) in added kernel code:" >&2
    printf '%s\n' "$debug_hits" >&2
    fail=1
fi

# ---------------------------------------------------------------------------
# Gate 4 (BG-TOL-001): the unscaled_legacy ratchet.
#
# ToleranceCtx::unscaled_legacy() is the Stage-A migration scaffold: it returns
# model_scale = 1.0, so a site migrated onto it keeps the legacy absolute
# tolerance and buys only the model/param judgement. Stage B replaces each one
# with a context carrying a real model scale. BG-TOL-001 is not discharged
# until none are left.
#
# A scaffold nobody removes is a permanent absolute tolerance -- the exact bug
# the item exists to kill -- so the count is ratcheted rather than trusted. It
# may never exceed the recorded ceiling, and the ceiling only ever moves down,
# which a Stage-B packet does explicitly in the same commit that removes the
# calls. Banning the constructor outright is not an option: that would ban
# Stage A, which is the only shardable way to make the judgement at all.
#
# Counted in production sources only (vendor/truck/*/src/**), excluding the
# file that defines it -- its own definition, doc comments and unit tests are
# not debt. Whole-tree at HEAD, deliberately not diff-scoped: the quantity
# under control is the total, and a diff-scoped version could not see a packet
# that added ten calls while another removed ten. HEAD, not the worktree, for
# the same reason the other gates read commits: V4 runs after the worker has
# committed, and an uncommitted call site is V0's business, not this gate's.
# ---------------------------------------------------------------------------
ceiling_file="scripts/unscaled_legacy_ceiling.txt"
if [ -f "$ceiling_file" ]; then
    ceiling=$(tr -cd '0-9' <"$ceiling_file")
    : "${ceiling:=0}"
    # `|| true` is load-bearing under `set -o pipefail`: git grep exits 1 when
    # it matches nothing, which is the healthy state here, and without it the
    # whole script died silently on a clean tree -- exit 1, no output, every
    # earlier gate unreported.
    legacy_count=$( { git grep -oh 'unscaled_legacy(' HEAD -- 'vendor/truck/*/src/*' \
        ':(exclude)vendor/truck/truck-base/src/tolerance.rs' 2>/dev/null || true; } | wc -l | tr -d ' ')
    : "${legacy_count:=0}"
    if [ "$legacy_count" -gt "$ceiling" ]; then
        echo "GATE-4 (BG-TOL-001): $legacy_count unscaled_legacy() call sites, ceiling is $ceiling." >&2
        echo "  unscaled_legacy is Stage-A scaffolding carrying the legacy absolute tolerance." >&2
        echo "  If this packet is a Stage-A migration shard, raise the ceiling in $ceiling_file" >&2
        echo "  in the same commit and say in the message how many sites it covers." >&2
        echo "  Otherwise thread a real model_scale instead -- the ceiling is not a target." >&2
        fail=1
    else
        echo "kernel-gates: GATE-4 unscaled_legacy $legacy_count/$ceiling."
    fi
fi

if [ "$fail" -eq 0 ]; then
    echo "kernel-gates: all P-3 gates pass."
else
    echo "kernel-gates: FAILED" >&2
    exit 1
fi
