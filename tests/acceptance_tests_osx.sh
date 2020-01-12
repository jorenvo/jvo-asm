#!/usr/bin/env bash
set -euo pipefail
source tests/acceptance_tests_common.sh

compile_and_compare_return 'mach' 'examples/exit_mach.jas' '99'

exit $FAILED