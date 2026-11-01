#!/usr/bin/env python3
"""UI interaction contract gate (specs/changes/*/interaction.md).

Single source of truth: a change's `interaction.md` declares baseline interaction
IDs with priority P0/P1; the real-browser suite tags each test
`@interaction:<ID>` and writes `target/evidence/<change>/interaction/<ID>.json`
on success. This gate checks contract -> tag -> evidence coverage plus test
anti-patterns (`waitForTimeout`, `force: true`, bare `test.skip`).

Enforced active contracts must also carry the sibling intuition files that
`docs/27` §2.7/§2.8 / `docs/11` §8.6 require: `intuition.md` with at least three
`INT-NN` table rows and a non-empty `intuition-walkthrough.md`. INT rows are
*not* tag/evidence gated on purpose: intuition quality is judged by the reviewer
walkthrough and user testing, not by this script.

Enforcement modes:
  * proposal status in {in-progress, done}: all `active` contracts enforced.
  * otherwise (draft/approved/missing) or `planned` contracts: WARN and skip
    (the change is not yet the team's committed work item).
  * `--strict` (or ZEOKEEL_UI_GATE_STRICT=1): enforce every `active` contract
    regardless of proposal status. `planned` contracts are still WARN-only.

Exit code 0 = no errors, 1 = errors (self-test failures too).
"""

from __future__ import annotations

import json
import os
import re
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

ID_RE = re.compile(r"\b([A-Z][A-Z0-9]{1,9}-[A-Z][A-Z0-9]{1,9}-\d{2})\b")
ID_ROW_RE = re.compile(r"([A-Z][A-Z0-9]{1,9}-[A-Z][A-Z0-9]{1,9}-\d{2})")
TAG_RE = re.compile(r"@interaction:([A-Z][A-Z0-9]{1,9}-[A-Z][A-Z0-9]{1,9}-\d{2})")
STATUS_RE = re.compile(r"状态\s*[:：]\s*`?(active|planned|draft|approved|in-progress|done)`?")
EVIDENCE_ROOT_RE = re.compile(r"证据根\s*[:：]\s*`([^`]+)`")
TEST_DIR_RE = re.compile(r"测试目录\s*[:：]\s*`([^`]+)`")

SCOPE_PRIORITIES = ("P0", "P1")
ENFORCED_PROPOSAL_STATUSES = ("in-progress", "done")
ANTI_PATTERNS = (
    ("waitForTimeout", re.compile(r"waitForTimeout")),
    ("force: true", re.compile(r"force\s*:\s*true")),
)
SKIP_RE = re.compile(r"\btest\.skip\b")
KNOWN_ISSUE_MARKER = "@known-issue"
INT_ROW_RE = re.compile(r"INT-\d{2}")
INTUITION_MIN_ROWS = 3
WALKTHROUGH_MIN_BYTES = 64


@dataclass
class Contract:
    path: Path
    status: str
    proposal_status: str
    evidence_root: Path
    test_dir: Path
    entries: dict[str, str] = field(default_factory=dict)

    @property
    def scope_ids(self) -> dict[str, str]:
        """P0/P1 ids gated by the contract (P2 is optional by definition)."""
        return {item_id: priority for item_id, priority in self.entries.items() if priority in SCOPE_PRIORITIES}


@dataclass
class GateResult:
    contracts: int
    enforced_contracts: int
    tests: int
    tags: int
    defined_ids: int
    enforced_ids: int
    intuition_contracts: int = 0
    intuition_rows: int = 0
    errors: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)


def rel(root: Path, path: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def _status_of(text: str) -> str:
    match = STATUS_RE.search(text)
    return "" if match is None else match.group(1)


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def _count_int_rows(path: Path) -> int:
    """Counts `intuition.md` table rows whose first cell is an `INT-NN` id."""
    rows = 0
    for line in _read_text(path).splitlines():
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if cells and INT_ROW_RE.fullmatch(cells[0]) is not None:
            rows += 1
    return rows


def parse_contract(path: Path, root: Path) -> Contract:
    text = _read_text(path)
    status = _status_of(text)
    proposal = _status_of(_read_text(path.parent / "proposal.md"))
    change_id = path.parent.name

    evidence_match = EVIDENCE_ROOT_RE.search(text)
    evidence_text = "target/evidence/%s/interaction" % change_id if evidence_match is None else evidence_match.group(1)
    test_match = TEST_DIR_RE.search(text)
    test_text = "e2e/interaction" if test_match is None else test_match.group(1)

    entries: dict[str, str] = {}
    for line in text.splitlines():
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 2:
            continue
        match = ID_ROW_RE.fullmatch(cells[0])
        if match is None:
            continue
        priority = next((cell for cell in cells[1:] if cell in ("P0", "P1", "P2")), "")
        entries.setdefault(match.group(1), priority or "P2")

    return Contract(
        path=path,
        status=status,
        proposal_status=proposal,
        evidence_root=(root / evidence_text).resolve(),
        test_dir=(root / test_text).resolve(),
        entries=entries,
    )


def _scan_tests(root: Path, contracts: list[Contract]) -> tuple[int, set[str], dict[str, list[str]], list[str]]:
    dirs: set[Path] = {root / "e2e" / "interaction"}
    for contract in contracts:
        dirs.add(contract.test_dir)
    files: set[Path] = set()
    for directory in dirs:
        if directory.is_dir():
            files.update(directory.rglob("*.ts"))
    files.update(root.glob("e2e/interaction/**/*.ts"))
    files = {test_file for test_file in files if test_file.is_file()}

    tags: set[str] = set()
    locations: dict[str, list[str]] = {}
    errors: list[str] = []
    for test_file in sorted(files):
        lines = _read_text(test_file).splitlines()
        for number, line in enumerate(lines, start=1):
            for match in TAG_RE.finditer(line):
                item_id = match.group(1)
                tags.add(item_id)
                locations.setdefault(item_id, []).append("%s:%d" % (rel(root, test_file), number))
            for label, pattern in ANTI_PATTERNS:
                if pattern.search(line):
                    errors.append(
                        "anti-pattern: %s in %s:%d (%s)"
                        % (label, rel(root, test_file), number, line.strip())
                    )
            if SKIP_RE.search(line) and KNOWN_ISSUE_MARKER not in line:
                errors.append(
                    "anti-pattern: test.skip without @known-issue in %s:%d (%s)"
                    % (rel(root, test_file), number, line.strip())
                )
    return len(files), tags, locations, errors


def _evidence_error(path: Path, display: str, item_id: str) -> str | None:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        return "invalid-evidence: %s (%s)" % (display, error)
    except json.JSONDecodeError as error:
        return "invalid-evidence: %s (not JSON: %s)" % (display, error)
    if not isinstance(data, dict):
        return "invalid-evidence: %s (expected a JSON object)" % display
    if data.get("id") != item_id:
        return "invalid-evidence: %s (id mismatch, expected %s)" % (display, item_id)
    if data.get("passed") is not True:
        return "invalid-evidence: %s (passed is not true)" % display
    return None


def run_gate(root: Path, strict: bool = False) -> GateResult:
    root = root.resolve()
    contract_paths = sorted(root.glob("specs/changes/*/interaction.md"))
    contracts = [parse_contract(path, root) for path in contract_paths]

    test_count, tags, _locations, errors = _scan_tests(root, contracts)
    warnings: list[str] = []

    for contract in contracts:
        if contract.status == "":
            errors.append(
                "missing-status: %s (declare `- 状态：`active|planned``)"
                % rel(root, contract.path)
            )
        elif contract.status not in ("active", "planned"):
            errors.append(
                "invalid-status: %s (status=%r, expected active|planned)"
                % (rel(root, contract.path), contract.status)
            )

    defined_ids: set[str] = set()
    for contract in contracts:
        defined_ids.update(contract.entries)
    for unknown in sorted(tags - defined_ids):
        errors.append(
            "unknown-id: test tag @interaction:%s is not defined in any interaction.md" % unknown
        )

    enforced_contracts = 0
    enforced_ids = 0
    intuition_contracts = 0
    intuition_rows = 0
    for contract in contracts:
        scope = contract.scope_ids
        enforce = contract.status == "active" and (
            strict or contract.proposal_status in ENFORCED_PROPOSAL_STATUSES
        )
        if not enforce:
            reason = "status=%s, proposal=%s" % (contract.status or "missing", contract.proposal_status or "missing")
            hint = " (use --strict to enforce now)" if contract.status == "active" else ""
            warnings.append(
                "planned-skip: %s (%s); %d P0/P1 id(s) not enforced%s"
                % (rel(root, contract.path), reason, len(scope), hint)
            )
            continue
        enforced_contracts += 1
        for item_id in sorted(scope):
            enforced_ids += 1
            if item_id not in tags:
                errors.append(
                    "missing-tag: %s has no @interaction:%s test in %s"
                    % (rel(root, contract.path), item_id, rel(root, contract.test_dir))
                )
            evidence = contract.evidence_root / ("%s.json" % item_id)
            display = rel(root, evidence)
            if not evidence.is_file():
                errors.append(
                    "missing-evidence: %s for active %s id %s (%s)"
                    % (display, scope[item_id], item_id, rel(root, contract.path))
                )
            else:
                problem = _evidence_error(evidence, display, item_id)
                if problem is not None:
                    errors.append("%s (%s)" % (problem, rel(root, contract.path)))

        # Intuition baseline (docs/27 §2.7/§2.8): existence + minimum shape only.
        # INT rows are intentionally not checked against @interaction tags/evidence.
        intuition_path = contract.path.parent / "intuition.md"
        walkthrough_path = contract.path.parent / "intuition-walkthrough.md"
        if not intuition_path.is_file():
            errors.append(
                "missing-intuition: %s (enforced active contract needs sibling intuition.md)"
                % rel(root, intuition_path)
            )
        else:
            intuition_contracts += 1
            rows = _count_int_rows(intuition_path)
            intuition_rows += rows
            if rows < INTUITION_MIN_ROWS:
                errors.append(
                    "intuition-too-few: %s has %d INT row(s), need >=%d"
                    % (rel(root, intuition_path), rows, INTUITION_MIN_ROWS)
                )
        if not walkthrough_path.is_file():
            errors.append(
                "missing-walkthrough: %s (enforced active contract needs a non-empty intuition-walkthrough.md)"
                % rel(root, walkthrough_path)
            )
        else:
            size = walkthrough_path.stat().st_size
            if size < WALKTHROUGH_MIN_BYTES:
                errors.append(
                    "missing-walkthrough: %s is effectively empty (%d bytes, need >=%d)"
                    % (rel(root, walkthrough_path), size, WALKTHROUGH_MIN_BYTES)
                )

    return GateResult(
        contracts=len(contracts),
        enforced_contracts=enforced_contracts,
        tests=test_count,
        tags=len(tags),
        defined_ids=len(defined_ids),
        enforced_ids=enforced_ids,
        intuition_contracts=intuition_contracts,
        intuition_rows=intuition_rows,
        errors=errors,
        warnings=warnings,
    )


def print_result(result: GateResult, strict: bool) -> None:
    print(
        "ui-interaction-gate: mode=%s checked contracts=%d (enforced=%d), tests=%d, tags=%d, "
        "ids=%d (enforced P0/P1=%d), intuition=%d contract(s)/%d row(s)"
        % (
            "strict" if strict else "proposal-status",
            result.contracts,
            result.enforced_contracts,
            result.tests,
            result.tags,
            result.defined_ids,
            result.enforced_ids,
            result.intuition_contracts,
            result.intuition_rows,
        )
    )
    for warning in result.warnings:
        print("WARN %s" % warning)
    for error in result.errors:
        print("ERROR %s" % error)
    if result.errors:
        print("FAILED: %d error(s), %d warning(s)" % (len(result.errors), len(result.warnings)))
    else:
        print("OK: all enforced active P0/P1 interactions are tagged and have passing evidence.")


def _write_fixture_file(root: Path, relative: str, content: str) -> None:
    path = root / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def _write_evidence(root: Path, directory: str, item_id: str) -> None:
    _write_fixture_file(
        root,
        "%s/%s.json" % (directory, item_id),
        json.dumps({"id": item_id, "title": item_id, "at": "2026-11-01T00:00:00Z", "passed": True}),
    )


def _write_intuition_fixture(root: Path, change: str, rows: int = 3, walkthrough: bool = True) -> None:
    table_rows = "\n".join(
        "| INT-%02d | discoverability | composer | send a message | controls are visible | hover-only | e2e | walkthrough |"
        % (index + 1)
        for index in range(rows)
    )
    _write_fixture_file(
        root,
        "specs/changes/%s/intuition.md" % change,
        "# fixture intuition baseline\n\n"
        "| ID | 原则 | surface | 场景 / top task | 用户直觉期望 | 反例 | 验证方式 | 证据 |\n"
        "| --- | --- | --- | --- | --- | --- | --- | --- |\n"
        "%s\n" % table_rows,
    )
    if walkthrough:
        _write_fixture_file(
            root,
            "specs/changes/%s/intuition-walkthrough.md" % change,
            "# fixture cognitive walkthrough\n\n"
            "persona P1 first-time user, P2 keyboard-heavy user, P3 assistive-tech user;\n"
            "top tasks T1/T2/T3 answer the four questions each step and keep a friction table.\n",
        )


def _contract_fixture(status: str, evidence: str, ids: list[tuple[str, str]]) -> str:
    rows = "\n".join(
        "| %s | pre | action | expected | %s | a.spec.ts | %s.json |" % (item_id, priority, item_id)
        for item_id, priority in ids
    )
    return (
        "# fixture interaction contract\n\n"
        "- 状态：`%s`\n"
        "- 证据根：`%s/`\n"
        "- 测试目录：`e2e/interaction/`\n\n"
        "| ID | 前置 | 人的动作 | 期望 | 优先级 | 测试落点 | 证据 |\n"
        "| --- | --- | --- | --- | --- | --- | --- |\n"
        "%s\n" % (status, evidence, rows)
    )


def _ensure(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def run_self_test() -> int:
    steps: list[str] = []
    try:
        return _run_self_test_body(steps)
    except AssertionError as error:
        print("[self-test] FAILED: %s" % error)
        return 1


def _run_self_test_body(steps: list[str]) -> int:
    with tempfile.TemporaryDirectory(prefix="ui-interaction-gate-self-test-") as temporary:
        root = Path(temporary)

        _write_fixture_file(root, "specs/changes/change-a/proposal.md", "# a\n\n- 状态：`in-progress`\n")
        _write_fixture_file(
            root,
            "specs/changes/change-a/interaction.md",
            _contract_fixture(
                "active",
                "target/evidence/change-a/interaction",
                [("AAA-AAA-01", "P0"), ("AAA-AAA-02", "P1"), ("AAA-AAA-03", "P2")],
            ),
        )
        _write_fixture_file(root, "specs/changes/change-b/proposal.md", "# b\n\n- 状态：`approved`\n")
        _write_fixture_file(
            root,
            "specs/changes/change-b/interaction.md",
            _contract_fixture(
                "planned",
                "target/evidence/change-b/interaction",
                [("BBB-BBB-01", "P0")],
            ),
        )
        _write_fixture_file(root, "specs/changes/change-c/proposal.md", "# c\n\n- 状态：`draft`\n")
        _write_fixture_file(
            root,
            "specs/changes/change-c/interaction.md",
            _contract_fixture(
                "active",
                "target/evidence/change-c/interaction",
                [("CCC-CCC-01", "P1")],
            ),
        )
        _write_fixture_file(
            root,
            "e2e/interaction/a.spec.ts",
            'test("a", { tag: ["@interaction:AAA-AAA-01"] }, async () => {});\n'
            'test("b", { tag: ["@interaction:AAA-AAA-02"] }, async () => {});\n',
        )
        _write_intuition_fixture(root, "change-a")
        _write_intuition_fixture(root, "change-c")
        _write_evidence(root, "target/evidence/change-a/interaction", "AAA-AAA-01")
        _write_evidence(root, "target/evidence/change-a/interaction", "AAA-AAA-02")

        result = run_gate(root, strict=False)
        _ensure(result.errors == [], "normal mode errors: %r" % result.errors)
        _ensure(result.enforced_contracts == 1, "only in-progress active contract is enforced")
        _ensure(any("change-b" in warning for warning in result.warnings), "planned contract must warn")
        _ensure(any("change-c" in warning for warning in result.warnings), "active+draft must warn")
        _ensure(not any("AAA-AAA-03" in error for error in result.errors), "P2 must not be gated")
        _ensure(result.intuition_contracts == 1, "normal mode counts 1 enforced intuition contract")
        _ensure(result.intuition_rows == 3, "normal mode counts 3 INT rows")
        steps.append("normal mode enforces in-progress active contract, warns planned/draft")

        strict_result = run_gate(root, strict=True)
        joined = "\n".join(strict_result.errors)
        _ensure("CCC-CCC-01" in joined, "strict mode must enforce active contract with draft proposal")
        _ensure("BBB-BBB-01" not in joined, "planned contract must stay warn-only in strict mode")
        steps.append("strict mode enforces active contracts regardless of proposal status")

        _write_evidence(root, "target/evidence/change-c/interaction", "CCC-CCC-01")
        _write_fixture_file(root, "e2e/interaction/c.spec.ts", 'test("c", { tag: ["@interaction:CCC-CCC-01"] }, async () => {});\n')
        strict_result = run_gate(root, strict=True)
        _ensure(strict_result.errors == [], "strict pass errors: %r" % strict_result.errors)
        _ensure(strict_result.intuition_contracts == 2, "strict mode counts both enforced intuition contracts")
        _ensure(strict_result.intuition_rows == 6, "strict mode counts both INT tables (INT rows are not tag/evidence gated)")
        steps.append("strict mode passes when evidence and tags exist (INT rows are not tag/evidence gated)")

        _write_fixture_file(
            root,
            "e2e/interaction/unknown.spec.ts",
            'test("u", { tag: ["@interaction:ZZZ-ZZZ-99"] }, async () => {});\n',
        )
        _ensure(any("unknown-id" in error for error in run_gate(root, strict=True).errors), "unknown tag must fail")
        steps.append("unknown test tag fails")

        (root / "e2e/interaction/unknown.spec.ts").unlink()
        _write_fixture_file(
            root,
            "e2e/interaction/anti.spec.ts",
            'test("a", async () => { await page.waitForTimeout(10); });\n'
            'test("b", async () => { await page.click("button"); });\n',
        )
        _ensure(any("waitForTimeout" in error for error in run_gate(root, strict=True).errors), "waitForTimeout must fail")
        steps.append("waitForTimeout anti-pattern fails")

        _write_fixture_file(
            root,
            "e2e/interaction/anti.spec.ts",
            'test("a", async () => { await page.click("button", { force: true }); });\n',
        )
        _ensure(any("force: true" in error for error in run_gate(root, strict=True).errors), "force: true must fail")
        steps.append("force: true anti-pattern fails")

        _write_fixture_file(root, "e2e/interaction/anti.spec.ts", 'test.skip("later", async () => {});\n')
        _ensure(any("test.skip" in error for error in run_gate(root, strict=True).errors), "bare test.skip must fail")
        _write_fixture_file(
            root,
            "e2e/interaction/anti.spec.ts",
            'test.skip("later", async () => {}); // @known-issue TRACKER-1\n',
        )
        _ensure(not any("test.skip" in error for error in run_gate(root, strict=True).errors), "@known-issue skip must pass")
        steps.append("bare test.skip fails, @known-issue skip passes")

        (root / "e2e/interaction/anti.spec.ts").unlink()
        _write_fixture_file(
            root,
            "e2e/interaction/a.spec.ts",
            'test("a", { tag: ["@interaction:AAA-AAA-01"] }, async () => {});\n',
        )
        _ensure(any("missing-tag" in error for error in run_gate(root, strict=True).errors), "missing tag must fail")
        steps.append("missing tag fails")

        (root / "target/evidence/change-a/interaction/AAA-AAA-01.json").unlink()
        _ensure(any("missing-evidence" in error for error in run_gate(root, strict=True).errors), "missing evidence must fail")
        steps.append("missing evidence fails")

        intuition_file = root / "specs/changes/change-a/intuition.md"
        intuition_file.unlink()
        _ensure(
            any("missing-intuition" in error for error in run_gate(root, strict=True).errors),
            "missing intuition.md must fail",
        )
        steps.append("missing intuition.md fails (missing-intuition)")

        _write_intuition_fixture(root, "change-a", rows=2)
        _ensure(
            any("intuition-too-few" in error for error in run_gate(root, strict=True).errors),
            "<3 INT rows must fail",
        )
        _write_intuition_fixture(root, "change-a", rows=3)
        _ensure(
            not any("intuition-too-few" in error for error in run_gate(root, strict=True).errors),
            "3 INT rows must pass",
        )
        steps.append("intuition.md with <3 INT rows fails (intuition-too-few)")

        walkthrough_file = root / "specs/changes/change-a/intuition-walkthrough.md"
        walkthrough_file.unlink()
        _ensure(
            any("missing-walkthrough" in error for error in run_gate(root, strict=True).errors),
            "missing walkthrough must fail",
        )
        walkthrough_file.write_text("", encoding="utf-8")
        _ensure(
            any("missing-walkthrough" in error for error in run_gate(root, strict=True).errors),
            "empty walkthrough must fail",
        )
        _write_intuition_fixture(root, "change-a", rows=3)
        steps.append("missing/empty intuition-walkthrough.md fails (missing-walkthrough)")

    for step in steps:
        print("[self-test] ok: %s" % step)
    print("[self-test] OK: %d checks" % len(steps))
    return 0


def main(argv: list[str]) -> int:
    args = list(argv[1:])
    if "--self-test" in args:
        return run_self_test()
    strict = "--strict" in args or os.environ.get("ZEOKEEL_UI_GATE_STRICT") == "1"
    root = Path.cwd()
    if "--root" in args:
        root = Path(args[args.index("--root") + 1]).resolve()
    result = run_gate(root, strict=strict)
    print_result(result, strict)
    return 0 if not result.errors else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
