#!/bin/bash
# Pre-push validation script
# Run this manually before pushing: ./scripts/pre-push.sh

set -e  # Exit on any error

echo "🔍 Pre-push validation - Running all CI checks locally..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print section headers
print_section() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✅ $2 passed${NC}\n"
    else
        echo -e "${RED}❌ $2 failed${NC}\n"
        exit 1
    fi
}

# Check if we're in a git repo
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}❌ Not in a git repository${NC}"
    exit 1
fi

print_section "Format Check"
echo -e "${YELLOW}Running cargo fmt --check...${NC}"
cargo fmt --check
print_status $? "Format check"

print_section "Clippy Linting"
echo -e "${YELLOW}Running cargo clippy --lib --bins -- -D warnings...${NC}"
cargo clippy --lib --bins -- -D warnings
print_status $? "Clippy linting"

print_section "All Tests"
echo -e "${YELLOW}Running cargo test --all-features --workspace...${NC}"
cargo test --all-features --workspace
print_status $? "All tests"

print_section "Release Build"
echo -e "${YELLOW}Running cargo build --release --lib --bins...${NC}"
cargo build --release --lib --bins
print_status $? "Release build"

print_section "Documentation"
echo -e "${YELLOW}Running cargo doc --no-deps --lib --bins...${NC}"
cargo doc --no-deps --lib --bins
print_status $? "Documentation build"

echo -e "${GREEN}🎉 All pre-push checks passed!${NC}"
echo -e "${GREEN}✨ Your code is ready to push to GitHub${NC}"
echo ""
echo -e "${BLUE}To push now: git push origin master${NC}"