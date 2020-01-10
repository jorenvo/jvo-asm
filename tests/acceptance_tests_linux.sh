#!/usr/bin/env bash
set -euo pipefail
source tests/acceptance_tests_common.sh

compile_and_compare_stdout 'elf' 'examples/print.jas' 'hi!'
compile_and_compare_return 'elf' 'examples/base_ptr_addressing.jas' '4'
compile_and_compare_return 'elf' 'examples/multiple_data_sections.jas' '6'
compile_and_compare_return 'elf' 'examples/find_max.jas' '222'
compile_and_compare_return 'elf' 'examples/square.jas' '49'
compile_and_compare_return 'elf' 'examples/factorial.jas' '120'

exit $FAILED
