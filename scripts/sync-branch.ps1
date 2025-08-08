#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Automatically sync local branch with remote, handling divergence through rebase
.DESCRIPTION
    This script handles common git pull issues when branches have diverged,
    particularly useful when CI/CD creates version bump commits while you're developing.
.PARAMETER Force
    Force sync even if there are uncommitted changes (will stash them)
.PARAMETER Branch
    Branch to sync (defaults to current branch)
.EXAMPLE
    .\sync-branch.ps1
    .\sync-branch.ps1 -Force
#>

param(
    [switch]$Force,
    [string]$Branch = ""
)

# Colors for output
$Host.UI.RawUI.ForegroundColor = "White"
function Write-Info { param($Message) Write-Host "ℹ️  $Message" -ForegroundColor Cyan }
function Write-Success { param($Message) Write-Host "✅ $Message" -ForegroundColor Green }
function Write-Warning { param($Message) Write-Host "⚠️  $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "❌ $Message" -ForegroundColor Red }

# Get current branch if not specified
if (-not $Branch) {
    $Branch = git rev-parse --abbrev-ref HEAD 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to get current branch"
        exit 1
    }
}

Write-Info "Syncing branch: $Branch"

# Check for uncommitted changes
$status = git status --porcelain
$hasChanges = $status -and $status.Trim() -ne ""
$stashed = $false

if ($hasChanges) {
    if ($Force) {
        Write-Warning "Stashing uncommitted changes..."
        $stashMessage = "Auto-stash before sync $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
        git stash push -m $stashMessage
        if ($LASTEXITCODE -eq 0) {
            $stashed = $true
            Write-Success "Changes stashed"
        }
    } else {
        Write-Error "You have uncommitted changes. Use -Force to stash them automatically."
        Write-Host "  Or commit/stash them manually first."
        exit 1
    }
}

# Fetch latest from remote
Write-Info "Fetching latest from origin..."
git fetch origin
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to fetch from origin"
    if ($stashed) {
        Write-Warning "Restoring stashed changes..."
        git stash pop
    }
    exit 1
}

# Check if branches have diverged
$localCommit = git rev-parse HEAD
$remoteCommit = git rev-parse "origin/$Branch" 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Warning "Remote branch origin/$Branch doesn't exist"
    Write-Success "Nothing to sync"
    if ($stashed) {
        Write-Info "Restoring stashed changes..."
        git stash pop
    }
    exit 0
}

$mergeBase = git merge-base HEAD "origin/$Branch"

if ($localCommit -eq $remoteCommit) {
    Write-Success "Already up to date!"
}
elseif ($localCommit -eq $mergeBase) {
    # We're behind, can fast-forward
    Write-Info "Fast-forwarding to origin/$Branch..."
    git pull --ff-only origin $Branch
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Successfully fast-forwarded"
    }
    else {
        Write-Error "Fast-forward failed"
    }
}
elseif ($remoteCommit -eq $mergeBase) {
    # We're ahead
    Write-Success "Your branch is ahead of origin/$Branch"
    Write-Info "You may want to push your changes: git push"
}
else {
    # Branches have diverged - need to rebase
    Write-Warning "Branches have diverged. Attempting rebase..."
    
    # Show what's different
    $localOnly = git log --oneline "$remoteCommit..$localCommit" 2>$null
    $remoteOnly = git log --oneline "$localCommit..$remoteCommit" 2>$null
    
    if ($localOnly) {
        Write-Info "Your local commits:"
        $localOnly | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
    }
    
    if ($remoteOnly) {
        Write-Info "Remote commits:"
        $remoteOnly | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
    }
    
    # Perform rebase
    Write-Info "Rebasing your commits on top of origin/$Branch..."
    git rebase "origin/$Branch"
    
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Successfully rebased!"
        
        # Check if we need to force push
        $needsForcePush = git status | Select-String "Your branch and .* have diverged"
        if ($needsForcePush) {
            Write-Warning "You'll need to force push: git push --force-with-lease"
        }
    } else {
        Write-Error "Rebase failed - likely due to conflicts"
        Write-Info "To resolve:"
        Write-Host "  1. Fix conflicts in the marked files" -ForegroundColor Gray
        Write-Host "  2. Stage resolved files: git add <files>" -ForegroundColor Gray
        Write-Host "  3. Continue rebase: git rebase --continue" -ForegroundColor Gray
        Write-Host "  Or abort: git rebase --abort" -ForegroundColor Gray
        
        if ($stashed) {
            Write-Warning "Note: You have stashed changes. Run 'git stash pop' after resolving."
        }
        exit 1
    }
}

# Restore stashed changes if any
if ($stashed) {
    Write-Info "Restoring stashed changes..."
    git stash pop
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Stashed changes restored"
    } else {
        Write-Warning "Failed to restore stashed changes. Run 'git stash pop' manually."
    }
}

Write-Success "Sync complete!"