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

compile_and_compare_stdout () {
    EXEC_FORMAT="${1}"
    SRC="${2}"
    EXPECTED="${3}"

    target/debug/jvo-asm "${EXEC_FORMAT}" "${SRC}"

    # the binary is allowed to return non-zero without stopping the tests
    set +e
    OUT=$(./a.out)
    set -e

    if [ "${OUT}" != "${EXPECTED}" ]; then
        fail "${SRC}" "${OUT}" "${EXPECTED}"
    fi
}

compile_and_compare_return () {
    EXEC_FORMAT="${1}"
    SRC="${2}"
    EXPECTED="${3}"

    target/debug/jvo-asm "${EXEC_FORMAT}" "${SRC}"

    # the binary is allowed to return non-zero without stopping the tests
    set +e
    ./a.out
    RETURN="${?}"
    set -e

    if [ "${RETURN}" -ne "${EXPECTED}" ]; then
        fail "${SRC}" "${RETURN}" "${EXPECTED}"
    fi
}

cargo build