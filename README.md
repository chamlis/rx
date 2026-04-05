# rx

Experimental reactivity in the shell.

`rx` takes an output command and an ordered list of input commands+arguments
(separated by `;`). We treat each line on the stdout of each input command as a
new value from it. `rx` waits for the first line from each input command, then
runs the output command, using these values as its arguments. Whenever one of
the input commands produces a new value (by outputting a new line), `rx` will
rerun the output command and incorporate the new value.

## Motivation

My motivation in writing `rx` was to make the program generating my `swaybar`
more event-driven. I considered switching from a shell-script to a more
featureful language, but wanted to experiment with a more Unix-style approach
and try to bring the core of reactivity to the shell itself.

For example, I can make my `swaybar` display both the current playing track and
the current time with a script like this:

```sh
rx echo \
    playerctl --follow -f '{{artist}} - {{title}}' metadata \; \
    sh -c 'while true; do date +"%Y-%m-%d %H:%M"; sleep 5; done'
```
