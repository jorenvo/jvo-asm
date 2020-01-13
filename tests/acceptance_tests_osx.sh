#!/usr/bin/env bash
set -euo pipefail
source tests/acceptance_tests_common.sh

compile_and_compare_return 'mach' 'examples/exit_mach.jas' '99'
compile_and_compare_return 'mach' 'examples/multiple_data_sections_mach.jas' '6'

exit $FAILED