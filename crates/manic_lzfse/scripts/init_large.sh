#!/bin/bash

# Init large files. Requires LZFSE reference binary.

# Default settings assume we are running from project root with:
# $ ./scripts/init_large.sh

get_zip () {
    f="${1##*/}"
    g="$2${f/.zip/.lzfse}"
    wget -qO- $1 | gunzip | lzfse -encode >> $g
    echo "$1 => $g"
}

get_zip "http://mattmahoney.net/dc/enwik8.zip" data/large/