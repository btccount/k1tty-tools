#!/usr/bin/env bash
set -u

for arg in "$@"; do
  if [ "$arg" = "-h" ] || [ "$arg" = "--help" ]; then
    echo "Usage: $(basename "$0") [port] [--no-open]"
    echo "Starts the K1tty Tools local web server (default port: 8780)."
    exit 0
  fi
done

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd "$SCRIPT_DIR" || exit 1

if [ -d "$SCRIPT_DIR/k1tty_editor/.venv" ]; then
  rm -rf "$SCRIPT_DIR/k1tty_editor/.venv"
fi

export PYTHONDONTWRITEBYTECODE=1
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"

if [ -t 1 ]; then
  printf '\033]0;K1tty Tools\007'
fi

pause_if_tty() {
  if [ -t 0 ]; then
    echo
    read -r -p "Press Enter to close..." _
  fi
}

has_deps() {
  "$1" -B -c "
import aiohttp, psutil
try:
    from compression import zstd
except ImportError:
    try:
        from backports import zstd
    except ImportError:
        import zstandard
" >/dev/null 2>&1
}

is_py310_plus() {
  "$1" -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)" >/dev/null 2>&1
}

RUN_PY=""
VENV_PY="$SCRIPT_DIR/.venv/bin/python"

if [ -x "$VENV_PY" ] && is_py310_plus "$VENV_PY" && has_deps "$VENV_PY"; then
  RUN_PY="$VENV_PY"
fi

if [ -z "$RUN_PY" ]; then
  CANDIDATES=()
  SEEN_PATHS=""

  add_candidate() {
    local p="$1"
    [ -n "$p" ] && [ -x "$p" ] || return 0
    local real
    real="$("$p" -c "import os, sys; print(os.path.realpath(sys.executable))" 2>/dev/null || echo "$p")"
    case ":$SEEN_PATHS:" in
      *":$real:"*) return 0 ;;
    esac
    if is_py310_plus "$p"; then
      SEEN_PATHS="$SEEN_PATHS:$real"
      CANDIDATES+=("$p")
    fi
  }

  for name in python3.14 python3.13 python3.12 python3.11 python3.10 python3; do
    if found="$(command -v "$name" 2>/dev/null)"; then
      add_candidate "$found"
    fi
    for dir in /opt/homebrew/bin /usr/local/bin /Library/Frameworks/Python.framework/Versions/*/bin /usr/bin; do
      [ -x "$dir/$name" ] && add_candidate "$dir/$name"
    done
  done

  if [ "${#CANDIDATES[@]}" -eq 0 ]; then
    echo "Python 3.10+ not found. Install Python 3 (e.g. 'brew install python' or https://www.python.org/downloads/) and try again."
    pause_if_tty
    exit 1
  fi

  for cand in "${CANDIDATES[@]}"; do
    if has_deps "$cand"; then
      RUN_PY="$cand"
      break
    fi
  done

  if [ -z "$RUN_PY" ]; then
    for cand in "${CANDIDATES[@]}"; do
      echo "Creating virtual environment using $cand..."
      rm -rf .venv
      if ! "$cand" -m venv .venv >/dev/null 2>&1; then
        rm -rf .venv
        continue
      fi
      echo "Installing dependencies from requirements.txt..."
      if .venv/bin/python -m pip install --disable-pip-version-check -r requirements.txt; then
        if has_deps ".venv/bin/python"; then
          RUN_PY="$VENV_PY"
          break
        fi
      fi
      rm -rf .venv
    done
  fi
fi

if [ -z "$RUN_PY" ]; then
  echo "Failed to install dependencies. Check your Python installation and internet connection."
  pause_if_tty
  exit 1
fi

HAS_PORT=0
for arg in "$@"; do
  case "$arg" in
    -*) ;;
    *) HAS_PORT=1; break ;;
  esac
done

if [ "$HAS_PORT" -eq 1 ]; then
  "$RUN_PY" -B -X utf8 server.py "$@"
else
  "$RUN_PY" -B -X utf8 server.py 8780 "$@"
fi
RC=$?
if [ "$RC" -ne 0 ]; then
  pause_if_tty
fi
exit "$RC"
