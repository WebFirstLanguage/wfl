#!/usr/bin/env python3
"""PRD format checker — enforcement arm of Docs/contributing/prd-process.md.

Red stub: accepts the documented arguments and reports nothing yet. The
tests in tests/tooling/test_check_prd_docs.py define the real contract.
"""

import argparse
import sys


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--base", default=None)
    parser.parse_args(argv)
    return 0


if __name__ == "__main__":
    sys.exit(main())
