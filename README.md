# Tomate

A Pomodoro timer for the CLI

## Install

Clone this repository and run `cargo install`.

## Usage

To do a Pomodoro:

1. Start a Pomodoro with `tomate start`
2. See remaining time with `tomate status`
3. End and archive the Pomodoro with `tomate finish`

To take a break:

1. Start a break with `tomate break`
2. See remaining time with `tomate status`
3. End the break with `tomate finish`

### Description and tags

Provide an optional argument to `start` to give the Pomodoro a description.
You can also add tags with the `--tags` (`-t`) option.

```console
$ tomate start -t work,fun "Do something cool"
$ tomate status
Current Pomodoro: Do something cool
Status: Active
Duration: 25m
Tags:
        - work
        - fun

Time remaining: 24:22

(use "tomate finish" to archive this Pomodoro)
(use "tomate clear" to delete this Pomodoro)
```

### History

The `tomate history` command shows you all the Pomodoros you've completed.

```console
$ tomate history
 Date Started  Duration  Tags         Description
 01 Apr 10:23       25m  work         Emails
 01 Apr 11:04       25m  home         Phone calls
 01 Apr 11:43       25m  work,boring  More stuff
```

### Hooks

Tomate can run commands when timers start and stop.
Create an executable script in the hooks directory (by default `${XDG_CONFIG_DIR}/tomate/hooks`)
with any of the following names:

- `pomodoro-start`
- `pomodoro-end`
- `shortbreak-start`
- `shortbreak-end`
- `longbreak-start`
- `longbreak-end`

Make sure they are executable (`chmod u+x pomodoro-start`).

## Acknowledgements

Many thanks to Justin Campbell for his [Open Pomodoro](https://github.com/open-pomodoro/openpomodoro-cli) project.
It's good enough to rewrite it in Rust.

## License

Copyright © 2025 Rosa Richter

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program.  If not, see <http://www.gnu.org/licenses/>.
