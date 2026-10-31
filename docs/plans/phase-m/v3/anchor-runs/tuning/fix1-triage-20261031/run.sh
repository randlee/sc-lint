#!/bin/sh
cd "$(dirname "$0")"
exec env -i PATH="$PATH" HOME="$HOME" TYPESAFE_API_KEY="$TYPESAFE_API_KEY" python3 -I run_bundle.py "$@"
