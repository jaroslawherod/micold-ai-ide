#!/usr/bin/env bash
#
# The quickstart §A.3 stream for feature 031: the SC-005 frame-time comparison (research R2).
#
# Prints 10,000 lines. Each holds one web address, and every tenth line is padded to exactly 1,000
# characters, so the stream is what SC-005 bounds: address-bearing output arriving fast, with long
# logical lines among it, while the pointer rests on a link.
#
#   stream-links.sh            every line holds an address
#   stream-links.sh --plain    the same lines with the address replaced by a word
#
# **The two modes differ in one thing only.** `--plain` swaps the address token for a token of the
# *same length* built only from `[a-z-]`, which `micold_core::link::detect` recognises as nothing
# (FR-001: no scheme, no `@`, no `/`, no `:`, no `.`). Every line is therefore byte-for-byte the same
# length in both modes, the same count of lines is written, and the padding and surrounding words are
# identical. So the only thing that can move between the two figures is the hover path — which is
# exactly the comparison §A.3 records, and the reason the token lengths are asserted below rather
# than trusted: a plain token one character shorter would change the wrapping of every padded line
# and land in the figure without appearing in it.
#
# Nothing here reads the clock or paces itself. The rate is whatever the pty and the client can take,
# which is the worst case SC-005 is about.
set -euo pipefail

PLAIN=0
for arg in "$@"; do
	case "$arg" in
	--plain) PLAIN=1 ;;
	-h | --help)
		sed -n '3,20p' "$0" | sed 's|^# \{0,1\}||'
		exit 0
		;;
	*)
		echo "unknown argument: $arg (expected --plain or nothing)" >&2
		exit 2
		;;
	esac
done

LINES=10000
# Every tenth line. 10,000 / 10 = 1,000 long lines.
LONG_EVERY=10
LONG_WIDTH=1000

# The address token, split so only the index varies: `https://example.com/docs/page-00042.html`.
ADDR_PRE='https://example.com/docs/page-'
ADDR_SUF='.html'
# Its plain counterpart, the same two lengths, over `[a-z-]` alone.
WORD_PRE='plain-words-here-no-address-x-'
WORD_SUF='-html'

# Asserted, not assumed: this is the one property that makes the two figures comparable.
if [ "${#ADDR_PRE}" -ne "${#WORD_PRE}" ] || [ "${#ADDR_SUF}" -ne "${#WORD_SUF}" ]; then
	echo "the plain token is not the address token's length; the two runs would not be comparable" >&2
	exit 1
fi

if [ "$PLAIN" -eq 1 ]; then
	PRE=$WORD_PRE
	SUF=$WORD_SUF
else
	PRE=$ADDR_PRE
	SUF=$ADDR_SUF
fi

# The padding for the long lines. Built once; sliced per line so every long line is exactly
# LONG_WIDTH characters whatever the index. Letters, hyphens and spaces only, so the padding itself
# is never recognised as an address in either mode.
FILLER=''
while [ "${#FILLER}" -lt "$LONG_WIDTH" ]; do
	FILLER+='padding-that-is-not-an-address '
done

i=1
while [ "$i" -le "$LINES" ]; do
	printf -v idx '%05d' "$i"
	line="line $idx: see $PRE$idx$SUF for details."
	if [ $((i % LONG_EVERY)) -eq 0 ]; then
		pad=$((LONG_WIDTH - ${#line}))
		if [ "$pad" -gt 0 ]; then
			line+=" ${FILLER:0:$((pad - 1))}"
		fi
	fi
	printf '%s\n' "$line"
	i=$((i + 1))
done
