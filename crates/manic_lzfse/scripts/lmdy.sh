#!/bin/bash

# Bulk dumps human readable LZ77 type LMD data usable by Plotty.
# We need a custom manic_lzfse build: navigate to `ring_lz_write::RingLzWriter`,
# uncomment the println statements following the Lmdy script comments and
# rebuild.

# Default settings assume we are running from project root with:
# $ ./scripts/lmdy.sh
LZFOO=target/release/lzfoo
FILES=data/snappy/*
DESTINATION=data/snappy/plotty_output/

mkdir -p $DESTINATION

for i in $FILES.lzfse ; do 
    f="${i##*/}"
    g="$DESTINATION${f/.lzfse/.lmd}"
    $LZFOO -decode -i $i -o /dev/null >> $g
    echo "$i => $g"
done
