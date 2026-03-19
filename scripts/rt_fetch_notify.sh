#!/usr/bin/env bash

source "${HOME}/.zshenv"

XDG_RUNTIME_DIR=/run/user/$(id -u)
export XDG_RUNTIME_DIR

if [ -z "${SSH_AUTH_SOCK}" ]
then
    SSH_AUTH_SOCK=$(find /tmp -path "/tmp/ssh-*/agent.*" -uid 1001 2> /dev/null)

    export SSH_AUTH_SOCK
fi

notify-send "Fetching repositories" --expire-time 20000

if [ -z "${SSH_AUTH_SOCK}" ]
then
    notify-send "ssh agent not started" --expire-time 10000 --urgency critical
fi

SUMMARY=$(rt fetch --quiet 2> "/tmp/rt_fetch.log")

notify-send "Fetching done" "${SUMMARY}" --expire-time 20000

SUMMARY=$(rt todo list | tail -n 1)

notify-send \
	"Repo TODO list" \
	"${SUMMARY}" \
	--expire-time 10000
