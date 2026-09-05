#!/bin/sh
set -eu
# Installer flow fixtures use synthetic archives; signature rejection has separate coverage.
[ "$#" -eq 9 ]
[ "$1 $2 $3 $4 $6 $8" = '-V -H -q -m -x -P' ]
[ -f "$5" ]
[ "$7" = "$5.minisig" ]
[ -f "$7" ]
[ -n "$9" ]
