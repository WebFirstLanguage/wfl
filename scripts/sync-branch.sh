#!/bin/bash
# sync-branch.sh - Automatically sync local branch with remote, handling divergence through rebase
#
# Usage:
#   ./sync-branch.sh           # Sync current branch
#   ./sync-branch.sh -f        # Force sync (stash uncommitted changes)
#   ./sync-branch.sh main      # Sync specific branch

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Functions for colored output
info() { echo -e "${CYAN}ℹ️  $1${NC}"; }
success() { echo -e "${GREEN}✅ $1${NC}"; }
warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
error() { echo -e "${RED}❌ $1${NC}"; }

# Parse arguments
FORCE=false
BRANCH=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--force)
            FORCE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS] [BRANCH]"
            echo "Options:"
            echo "  -f, --force    Force sync even with uncommitted changes (will stash)"
            echo "  -h, --help     Show this help message"
            echo "Arguments:"
            echo "  BRANCH         Branch to sync (defaults to current branch)"
            exit 0
            ;;
        *)
            BRANCH="$1"
            shift
            ;;
    esac
done

# Get current branch if not specified
if [ -z "$BRANCH" ]; then
    BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null)
    if [ $? -ne 0 ]; then
        error "Failed to get current branch"
        exit 1
    fi
fi

info "Syncing branch: $BRANCH"

# Check for uncommitted changes
STATUS=$(git status --porcelain)
HAS_CHANGES=false
STASHED=false

if [ -n "$STATUS" ]; then
    HAS_CHANGES=true
fi

if [ "$HAS_CHANGES" = true ]; then
    if [ "$FORCE" = true ]; then
        warning "Stashing uncommitted changes..."
        STASH_MSG="Auto-stash before sync $(date '+%Y-%m-%d %H:%M:%S')"
        if git stash push -m "$STASH_MSG"; then
            STASHED=true
            success "Changes stashed"
        fi
    else
        error "You have uncommitted changes. Use -f to stash them automatically."
        echo "  Or commit/stash them manually first."
        exit 1
    fi
fi

# Function to restore stash on exit
restore_stash() {
    if [ "$STASHED" = true ]; then
        info "Restoring stashed changes..."
        if git stash pop; then
            success "Stashed changes restored"
        else
            warning "Failed to restore stashed changes. Run 'git stash pop' manually."
        fi
    fi
}

# Fetch latest from remote
info "Fetching latest from origin..."
if ! git fetch origin; then
    error "Failed to fetch from origin"
    restore_stash
    exit 1
fi

# Check if branches have diverged
LOCAL_COMMIT=$(git rev-parse HEAD)
REMOTE_COMMIT=$(git rev-parse "origin/$BRANCH" 2>/dev/null)

if [ $? -ne 0 ]; then
    warning "Remote branch origin/$BRANCH doesn't exist"
    success "Nothing to sync"
    restore_stash
    exit 0
fi

MERGE_BASE=$(git merge-base HEAD "origin/$BRANCH")

if [ "$LOCAL_COMMIT" = "$REMOTE_COMMIT" ]; then
    success "Already up to date!"
elif [ "$LOCAL_COMMIT" = "$MERGE_BASE" ]; then
    # We're behind, can fast-forward
    info "Fast-forwarding to origin/$BRANCH..."
    if git pull --ff-only origin "$BRANCH"; then
        success "Successfully fast-forwarded"
    else
        error "Fast-forward failed"
    fi
elif [ "$REMOTE_COMMIT" = "$MERGE_BASE" ]; then
    # We're ahead
    success "Your branch is ahead of origin/$BRANCH"
    info "You may want to push your changes: git push"
else
    # Branches have diverged - need to rebase
    warning "Branches have diverged. Attempting rebase..."
    
    # Show what's different
    LOCAL_ONLY=$(git log --oneline "$REMOTE_COMMIT..$LOCAL_COMMIT" 2>/dev/null)
    REMOTE_ONLY=$(git log --oneline "$LOCAL_COMMIT..$REMOTE_COMMIT" 2>/dev/null)
    
    if [ -n "$LOCAL_ONLY" ]; then
        info "Your local commits:"
        echo "$LOCAL_ONLY" | while IFS= read -r line; do
            echo "  $line"
        done
    fi
    
    if [ -n "$REMOTE_ONLY" ]; then
        info "Remote commits:"
        echo "$REMOTE_ONLY" | while IFS= read -r line; do
            echo "  $line"
        done
    fi
    
    # Perform rebase
    info "Rebasing your commits on top of origin/$BRANCH..."
    if git rebase "origin/$BRANCH"; then
        success "Successfully rebased!"
        
        # Check if we need to force push
        if git status | grep -q "Your branch and .* have diverged"; then
            warning "You'll need to force push: git push --force-with-lease"
        fi
    else
        error "Rebase failed - likely due to conflicts"
        info "To resolve:"
        echo "  1. Fix conflicts in the marked files"
        echo "  2. Stage resolved files: git add <files>"
        echo "  3. Continue rebase: git rebase --continue"
        echo "  Or abort: git rebase --abort"
        
        if [ "$STASHED" = true ]; then
            warning "Note: You have stashed changes. Run 'git stash pop' after resolving."
        fi
        exit 1
    fi
fi

# Restore stashed changes if any
restore_stash

success "Sync complete!"