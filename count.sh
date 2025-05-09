#!/bin/bash

FILE="$1"

if [[ -z "$FILE" || ! -f "$FILE" ]]; then
    echo "Usage: $0 <filename>"
    exit 1
fi

moved_count=$(grep -c '^moved' "$FILE")
skipped_count=$(grep -c '^skipped' "$FILE")

echo "Moved: $moved_count"
echo "Skipped: $skipped_count"