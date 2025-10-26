#!/bin/bash
SRC=$1

i=0
while IFS= read -r line; do
    stdbuf -oL echo "$line"
    ((i++))
    if (( i % 5 == 0 )); then
        pause=$((2 + RANDOM % 4))  # pause aléatoire entre 2 et 5 s
        sleep "$pause"
    fi
done < "$SRC"
