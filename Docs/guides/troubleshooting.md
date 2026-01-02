# Troubleshooting Guide

## Environment Dump
To assist support teams in diagnosing issues, WFL provides a built-in environment dump feature.
Run the following command to generate a report of your current setup:

```bash
wfl --dump-env
```

This will output:
- WFL Version
- Build-time Rust Version
- OS and Architecture
- WFL LSP Server status
- Current loaded Configuration
- WFL Environment Variables

To save the output to a file:

```bash
wfl --dump-env --output env_dump.txt
```
