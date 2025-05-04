#!/bin/bash
set -e

# Compile the application in release mode
cargo build --release

# Check if nfpm is installed
if ! command -v nfpm &> /dev/null; then
    echo "nfpm is not installed. Installing..."
    
    # For Linux, you can use the following command:
    # curl -sfL https://install.goreleaser.com/github.com/goreleaser/nfpm.sh | sh

    # for Alpine Linux
    apk --no-cache add nfpm

    echo "Please install nfpm manually from https://github.com/goreleaser/nfpm"
    exit 1
fi

# Create RPM package
nfpm package --packager rpm --target ./dist/

echo "RPM package created in the ./dist/ directory" 