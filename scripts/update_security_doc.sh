#!/bin/bash
# Script to automatically update SECURITY.md with current version information from Cargo.toml

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"
SECURITY_MD="$PROJECT_ROOT/SECURITY.md"

echo -e "${CYAN}Updating SECURITY.md with current version information...${NC}"

# Check if files exist
if [ ! -f "$CARGO_TOML" ]; then
    echo -e "${RED}Error: Cargo.toml not found at $CARGO_TOML${NC}"
    exit 1
fi

if [ ! -f "$SECURITY_MD" ]; then
    echo -e "${RED}Error: SECURITY.md not found at $SECURITY_MD${NC}"
    exit 1
fi

# Extract version from Cargo.toml
VERSION_LINE=$(grep '^version = ' "$CARGO_TOML" | head -n 1)
if [[ $VERSION_LINE =~ version[[:space:]]*=[[:space:]]*\"([0-9]+)\.([0-9]+)\.([0-9]+)\" ]]; then
    CURRENT_YEAR="${BASH_REMATCH[1]}"
    CURRENT_MONTH="${BASH_REMATCH[2]}"
    CURRENT_BUILD="${BASH_REMATCH[3]}"
    CURRENT_VERSION="$CURRENT_YEAR.$CURRENT_MONTH.$CURRENT_BUILD"
    CURRENT_VERSION_PATTERN="$CURRENT_YEAR.$CURRENT_MONTH.x"
else
    echo -e "${RED}Error: Could not extract version from Cargo.toml${NC}"
    exit 1
fi

echo -e "${GREEN}Current version: $CURRENT_VERSION${NC}"

# Calculate previous months for support tiers
YEAR=$((10#$CURRENT_YEAR))
MONTH=$((10#$CURRENT_MONTH))

# Previous month (limited support)
PREV_MONTH=$((MONTH - 1))
PREV_YEAR=$YEAR
if [ $PREV_MONTH -lt 1 ]; then
    PREV_MONTH=12
    PREV_YEAR=$((YEAR - 1))
fi
LIMITED_SUPPORT_PATTERN="$PREV_YEAR.$PREV_MONTH.x"

# Two months ago (no support)
OLD_MONTH=$((MONTH - 2))
OLD_YEAR=$YEAR
if [ $OLD_MONTH -lt 1 ]; then
    OLD_MONTH=$((OLD_MONTH + 12))
    OLD_YEAR=$((YEAR - 1))
fi
if [ $OLD_MONTH -lt 1 ]; then
    OLD_MONTH=$((OLD_MONTH + 12))
    OLD_YEAR=$((OLD_YEAR - 1))
fi
NO_SUPPORT_PATTERN="$OLD_YEAR.$OLD_MONTH.x and older"

# Current date for "Last Updated"
CURRENT_DATE=$(date '+%B %Y')

# Current year for copyright
COPYRIGHT_YEAR="20$CURRENT_YEAR"

echo -e "${YELLOW}Support tiers:${NC}"
echo -e "  ${GREEN}Current:  $CURRENT_VERSION_PATTERN${NC}"
echo -e "  ${YELLOW}Limited:  $LIMITED_SUPPORT_PATTERN${NC}"
echo -e "  ${RED}None:     $NO_SUPPORT_PATTERN${NC}"

# Create temporary file for updates
TMP_FILE=$(mktemp)

# Read and update SECURITY.md
cat "$SECURITY_MD" | \
# Update version table
sed -E '/\| Version Pattern \| Supported/,/\| [0-9]+\.[0-9]+\.x and older\| ❌ No/{
    s/\| [0-9]+\.[0-9]+\.x \(Current\)\| ✅ Yes/| '"$CURRENT_VERSION_PATTERN"' (Current)| ✅ Yes/
    s/\| [0-9]+\.[0-9]+\.x[[:space:]]+\| ⚠️ Limited/| '"$LIMITED_SUPPORT_PATTERN"'         | ⚠️ Limited/
    s/\| [0-9]+\.[0-9]+\.x and older\| ❌ No/| '"$NO_SUPPORT_PATTERN"'| ❌ No/
}' | \
# Update footer
sed -E "s/\*\*Last Updated\*\*: [^\n]+/**Last Updated**: $CURRENT_DATE/" | \
sed -E "s/\*\*Version\*\*: [0-9]+\.[0-9]+\.[0-9]+/**Version**: $CURRENT_VERSION/" | \
sed -E "s/© [0-9]{4} Logbie LLC\./© $COPYRIGHT_YEAR Logbie LLC./" > "$TMP_FILE"

# Replace original file
mv "$TMP_FILE" "$SECURITY_MD"

echo -e "\n${GREEN}✓ SECURITY.md updated successfully!${NC}"
echo -e "  ${CYAN}Version: $CURRENT_VERSION${NC}"
echo -e "  ${CYAN}Date: $CURRENT_DATE${NC}"
echo -e "\n${YELLOW}Review the changes and commit if they look correct.${NC}"
