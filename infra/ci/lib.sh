# Shared bash helpers for the Aether dark-build runner.
# Source this from dark-build.sh and other runner scripts.

set -euo pipefail

log() {
    # Use printf for portability; ISO-8601-style timestamp.
    printf '[%s] %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"
}

require_command() {
    local cmd="$1"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        log "ERROR: required command not found: $cmd"
        return 127
    fi
}

# Phase 0 dark-build check: did any step attempt a network operation?
# Phase 1 replaces this with OS-level firewall assertions.
audit_network() {
    local log_dir="$1"
    if [[ ! -d "$log_dir" ]]; then
        log "no log dir to audit ($log_dir); skipping network audit"
        return 0
    fi
    # Best-effort: grep for common network clients in captured output.
    if grep -rE 'curl |wget |fetch |http\.get|requests\.get|reqwest::get' "$log_dir" >/dev/null 2>&1; then
        log "ERROR: dark-build attempted a network operation (curl/wget/...)"
        return 30
    fi
    return 0
}
