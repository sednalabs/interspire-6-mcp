#!/usr/bin/env python3
"""Check private Interspire source against the MCP compatibility profile.

This script is intentionally safe for a public repository: it accepts a local
Interspire source tree, checks for reviewed route/form/API contract markers,
and emits aggregate JSON only. It must not print proprietary source snippets.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class ContractCheck:
    area: str
    name: str
    relative_path: str
    patterns: tuple[str, ...]


CHECKS: tuple[ContractCheck, ...] = (
    ContractCheck(
        area="lists",
        name="list create/read/apply routes",
        relative_path="admin/functions/lists.php",
        patterns=(
            r"case\s+['\"]create['\"]",
            r"case\s+['\"]addlist['\"]",
            r"function\s+CreateList\s*\(",
            r"function\s+AddList\s*\(",
            r"\$GLOBALS\[['\"]Action['\"]\]\s*=\s*['\"]AddList['\"]",
        ),
    ),
    ContractCheck(
        area="lists",
        name="list metadata fields",
        relative_path="admin/functions/lists.php",
        patterns=(
            r"['\"]Name['\"]",
            r"['\"]OwnerName['\"]",
            r"['\"]OwnerEmail['\"]",
            r"['\"]ReplyToEmail['\"]",
            r"requestGetPOST\(['\"]BounceEmail['\"]",
            r"requestGetPOST\(['\"]UnsubscribeMailto['\"]",
            r"requestGetPOST\(['\"]NotifyOwner['\"]",
            r"requestGetPOST\(['\"]VisibleFields['\"]",
            r"requestGetPOST\(['\"]AvailableFields['\"]",
            r"requestGetPOST\(['\"]total_webhooks['\"]",
            r"requestGetPOST\(['\"]WebhookUrl_",
            r"requestGetPOST\(['\"]webhook_event_",
            r"requestGetPOST\(['\"]bounce_process['\"]",
        ),
    ),
    ContractCheck(
        area="lists",
        name="list form template",
        relative_path="admin/com/templates/lists_form.tpl",
        patterns=(
            r"name=['\"]frmListEditor['\"]",
            r"action=['\"]index\.php\?Page=Lists&Action=%%GLOBAL_Action%%['\"]",
            r"name=['\"]Name['\"]",
            r"name=['\"]OwnerName['\"]",
            r"name=['\"]OwnerEmail['\"]",
            r"name=['\"]ReplyToEmail['\"]",
            r"name=['\"]BounceEmail['\"]",
            r"name=['\"]VisibleFields\[\]['\"]",
            r"name=['\"]AvailableFields\[\]['\"]",
            r"%%GLOBAL_webhook_data%%",
        ),
    ),
    ContractCheck(
        area="campaigns",
        name="campaign management routes",
        relative_path="admin/functions/newsletters.php",
        patterns=(
            r"case\s+['\"]copy['\"]",
            r"case\s+['\"]edit['\"]",
            r"case\s+['\"]create['\"]",
            r"function\s+EditNewsletter\s*\(",
            r"function\s+CreateNewsletter\s*\(",
        ),
    ),
    ContractCheck(
        area="send",
        name="send wizard boundaries",
        relative_path="admin/functions/send.php",
        patterns=(
            r"function\s+Process\s*\(",
            r"Step2",
            r"Step3",
            r"Step4",
            r"Schedule",
        ),
    ),
    ContractCheck(
        area="xml",
        name="xml front controller",
        relative_path="admin/com/xml.php",
        patterns=(
            r"requesttype",
            r"requestmethod",
            r"username",
            r"usertoken",
            r"php://input",
        ),
    ),
    ContractCheck(
        area="xml",
        name="xml policy allowlist",
        relative_path="admin/com/xml_allowlist.php",
        patterns=(
            r"authentication",
            r"lists",
            r"subscribers",
        ),
    ),
)


def read_text(root: Path, relative_path: str) -> str | None:
    path = root / relative_path
    try:
        return path.read_text(encoding="utf-8", errors="ignore")
    except FileNotFoundError:
        return None


def check_patterns(text: str | None, patterns: Iterable[str]) -> tuple[bool, list[str]]:
    if text is None:
        return False, ["missing_file"]
    missing = [pattern for pattern in patterns if re.search(pattern, text, re.IGNORECASE) is None]
    return not missing, missing


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check a private Interspire source tree against public MCP compatibility contracts."
    )
    parser.add_argument(
        "--source-root",
        default=os.environ.get("INTERSPIRE_SOURCE_ROOT"),
        help="Path to private Interspire source root. Defaults to INTERSPIRE_SOURCE_ROOT.",
    )
    parser.add_argument(
        "--pretty",
        action="store_true",
        help="Pretty-print JSON.",
    )
    args = parser.parse_args()
    if not args.source_root:
        print(
            json.dumps(
                {
                    "ok": False,
                    "error": "source_root_required",
                    "message": "Pass --source-root or set INTERSPIRE_SOURCE_ROOT.",
                }
            ),
            file=sys.stderr,
        )
        return 2

    root = Path(args.source_root).expanduser().resolve()
    results = []
    for check in CHECKS:
        text = read_text(root, check.relative_path)
        ok, missing = check_patterns(text, check.patterns)
        results.append(
            {
                "area": check.area,
                "name": check.name,
                "relative_path": check.relative_path,
                "ok": ok,
                "pattern_count": len(check.patterns),
                "missing_count": len(missing),
                "missing_patterns": missing,
            }
        )

    failed = [result for result in results if not result["ok"]]
    payload = {
        "ok": not failed,
        "source_root_checked": str(root),
        "checks": len(results),
        "passed": len(results) - len(failed),
        "failed": len(failed),
        "results": results,
        "output_policy": "aggregate contract status only; no proprietary source snippets emitted",
    }
    print(json.dumps(payload, indent=2 if args.pretty else None, sort_keys=True))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
