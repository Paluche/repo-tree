#!/usr/bin/env bash

USER=$(id -u)

export XDG_RUNTIME_DIR="/run/user/${USER}"

notify-send "Fetching repositories" --expire-time 10000

SUMMARY=$(rt fetch --quiet 2> /dev/null)

notify-send \
  "Fetching done" \
  "${SUMMARY}" \
  --expire-time 10000
