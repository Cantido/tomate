---
title: TOMATE
section: 1
date: December 10, 2024
---

# NAME

tomate - Terminal-based Pomodoro timer

# SYNOPSIS

**tomate**
\[-c _path_ | -\-config _path_]
\[-h | -\-help]
\[-v | -\-verbose]
\[-V | -\-version]
_command_ \[_args_]

# DESCRIPTION

Tomate is a command-line Pomodoro client that supports task tagging, history tracking, and system integration via shell hooks.


# OPTIONS

-c *path*, -\-config *path*

: Use the config file at *path* instead of the default


-h, -\-help

: Print help


-v, -\-verbose

: Increase logging verbosity


-V, -\-version

: Print version


# COMMANDS

tomate-status(1)

: Show the current Pomodoro, if any

tomate-start(1)

: Start a Pomodoro timer

tomate-clear(1)

: Remove the existing Pomodoro timer, if any

tomate-finish(1)

: Stop and archive the current Pomodoro timer

tomate-break(1)

: Start a break timer

tomate-history(1)

: Print a list of all logged Pomorodo timers

tomate-purge(1)

: Delete all state and configuration files

tomate-help(1)

: Print a help message

# FILES

${XDG_CONFIG_HOME}/tomate/config.toml

: Configuration file.

${XDG_CONFIG_HOME}/tomate/hooks

: Script hooks to be executed on certain events. Currently `start`, `stop`, and `break` hooks are supported.

${XDG_STATE_HOME}/tomate/current.toml

: Present if a Pomodoro is currently active. Contains tags and the time the current Pomodoro was started.

${XDG_DATA_HOME}/tomate/history.toml

: Record of past Pomodoros.
