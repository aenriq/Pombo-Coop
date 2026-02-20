#!/usr/bin/env bash
set -euo pipefail

exec cargo run -p agent_manager_ui "$@"
