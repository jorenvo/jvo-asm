#!/usr/bin/env bash
set -euo pipefail

FAILED=0

fail () {
    FILENAME="${1}"
    RESULT="${2}"
    EXPECTED="${3}"

    FAILED=1

    echo "${FILENAME} resulted in ${RESULT}, expected ${EXPECTED}."
}

cargo build
target/debug/jvo-asm examples/print.jas
OUT=$(./a.out)
EXPECTED="abc"

if [ "${OUT}" != "${EXPECTED}" ]; then
    fail 'print.jas' "${OUT}" "${EXPECTED}"
fi

exit $FAILED
