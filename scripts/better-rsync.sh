#!/usr/bin/env bash

rsync --info=progress2 $@ | tr '\r' '\n' | awk '
BEGIN { last = -1 }
/%/ {
    if (match($0, /([0-9]+(\.[0-9]+)?)%/, m)) {
        p = int(m[1] + 0)
        if (p != last) {
            printf "%d of 100 steps (%d%%) done\n", p, p
            last = p
        }
    }
}
'