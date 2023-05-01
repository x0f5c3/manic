#!/bin/bash

# Bulk generate SHA256 hash. Requires LZFSE reference binary.

# Default settings assume we are running from project root with:
# $ ./scripts/hash.sh

sum () {
    for i in $1*.lzfse ; do 
        f="${i/lzfse/hash}"
        h=`lzfse -decode -i $i | sha256sum -b | head -c 64`
        echo $h | xxd -r -p > $f
        echo "$h $i"
    done
}

sum data/snappy/
sum data/large/
sum data/mutate/
sum data/special/
