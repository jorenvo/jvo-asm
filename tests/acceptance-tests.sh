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
    SRC="${1}"
    EXPECTED="${2}"

    target/debug/jvo-asm "${SRC}"

    # the binary is allowed to return non-zero without stopping the tests
    set +e
    OUT=$(./a.out)
    set -e

    if [ "${OUT}" != "${EXPECTED}" ]; then
        fail "${SRC}" "${OUT}" "${EXPECTED}"
    fi
}

compile_and_compare_return () {
    SRC="${1}"
    EXPECTED="${2}"

    target/debug/jvo-asm "${SRC}"

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
compile_and_compare_stdout 'examples/print.jas' 'hi!'
compile_and_compare_return 'examples/base_ptr_addressing.jas' '4'
compile_and_compare_return 'examples/multiple_data_sections.jas' '6'
compile_and_compare_return 'examples/find_max.jas' '222'
compile_and_compare_return 'examples/square.jas' '49'
compile_and_compare_return 'examples/factorial.jas' '120'

exit $FAILED
