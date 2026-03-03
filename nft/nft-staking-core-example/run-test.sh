#!/bin/bash
# Wrapper script for running tests with Surfpool
# (Reset network logic removed because the test dynamically handles time offset now)

echo "🚀 Running anchor test..."
anchor test --skip-local-validator "$@"
