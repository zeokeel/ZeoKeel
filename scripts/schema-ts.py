#!/usr/bin/env python3
"""Generate TS types + embedded JSON Schemas from shared/contracts/schema.

Single source of truth: Rust contract crates (`schemars`) exported by
`cargo test --workspace export_schema` into `shared/contracts/schema/*.json`.

Output `shared/contracts/ts/contracts.ts` contains:

* TS type declarations for every schema root and its `definitions`
  (`$ref` -> interface/type name, enums -> literal unions, tagged enums ->
  discriminated unions);
* one `<key>Schema` const per schema file (embedded JSON, Ajv input);
* a `SCHEMAS` map keyed by schema file (dash -> underscore) for runtime Ajv
  validation (C10/T22).

Output is deterministic: files/declarations/schema keys are sorted and the
embedded JSON is canonicalized with sorted object keys.
"""
from __future__ import annotations

import json
import pathlib
import re
import sys
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "shared/contracts/schema"
OUT = ROOT / "shared/contracts/ts/contracts.ts"

IDENTIFIER = re.compile(r"^[A-Za-z_$][A-Za-z0-9_$]*$")
VALUE_TYPES = {
    "string": "string",
    "integer": "number",
    "number": "number",
    "boolean": "boolean",
    "null": "null",
}
# Keys that describe a schema node rather than one of its properties.
NON_BODY_KEYS = ("$schema", "title", "definitions", "$defs")


class SchemaError(Exception):
    """Invalid or unsupported schema input."""


def sanitize(name: str) -> str:
    """Turn a schema title/definition name into a TS identifier."""
    ident = re.sub(r"[^A-Za-z0-9_$]+", "_", name).strip("_")
    if ident == "":
        return "Unnamed"
    if ident[0].isdigit():
        ident = "T" + ident
    return ident


def canonical(value: Any) -> str:
    """Canonical JSON for schema-body comparison (deterministic)."""
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def body_key(schema: dict[str, Any]) -> str:
    """Canonical schema body ignoring root-only keys (dedupe across files)."""
    body = {key: value for key, value in schema.items() if key not in NON_BODY_KEYS}
    return canonical(body)


def literal(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return repr(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    return "unknown"


def wrap_union_member(text: str) -> str:
    return f"({text})" if "&" in text else text


def wrap_intersection_member(text: str) -> str:
    return f"({text})" if "|" in text else text


def union(parts: list[str]) -> str:
    seen: set[str] = set()
    unique: list[str] = []
    for part in parts:
        if part in ("", "never"):
            continue
        if part not in seen:
            seen.add(part)
            unique.append(part)
    if not unique:
        return "never"
    if len(unique) == 1:
        return unique[0]
    return " | ".join(wrap_union_member(part) for part in unique)


def intersection(parts: list[str]) -> str:
    seen: set[str] = set()
    unique: list[str] = []
    for part in parts:
        if part in ("", "unknown"):
            continue
        if part not in seen:
            seen.add(part)
            unique.append(part)
    if not unique:
        return "unknown"
    if len(unique) == 1:
        return unique[0]
    return " & ".join(wrap_intersection_member(part) for part in unique)


def ref_type(reference: str, names: dict[str, str]) -> str:
    prefix = "#/definitions/"
    if reference.startswith(prefix):
        key = reference[len(prefix) :]
    elif reference.startswith("#/$defs/"):
        key = reference[len("#/$defs/") :]
    else:
        return "unknown"
    return names.get(key, sanitize(key))


def object_type(schema: dict[str, Any], names: dict[str, str], indent: int) -> str:
    properties = schema.get("properties", {})
    required = set(schema.get("required", []))
    lines: list[str] = []
    for key, prop in properties.items():
        if not isinstance(prop, dict):
            prop = {}
        optional = "" if key in required else "?"
        label = key if IDENTIFIER.match(key) else json.dumps(key)
        lines.append(f"{' ' * (indent + 2)}{label}{optional}: {ts_type(prop, names, indent + 2)};")
    parts: list[str] = []
    if lines:
        parts.append("{\n" + "\n".join(lines) + f"\n{' ' * indent}}}")
    additional = schema.get("additionalProperties")
    if isinstance(additional, dict):
        parts.append(f"Record<string, {ts_type(additional, names, indent)}>")
    elif additional is True and not lines:
        parts.append("Record<string, unknown>")
    if not parts:
        return "Record<string, unknown>"
    return " & ".join(parts)


def ts_type(node: Any, names: dict[str, str], indent: int = 0) -> str:
    """Render one JSON Schema node as a TS type expression."""
    if node is True:
        return "unknown"
    if node is False:
        return "never"
    if not isinstance(node, dict):
        return "unknown"
    if "$ref" in node and isinstance(node["$ref"], str):
        return ref_type(node["$ref"], names)
    if "const" in node:
        return literal(node["const"])
    if isinstance(node.get("enum"), list):
        return union([literal(item) for item in node["enum"]])
    if isinstance(node.get("allOf"), list):
        return intersection([ts_type(item, names, indent) for item in node["allOf"]])
    combined = node.get("anyOf")
    if not isinstance(combined, list):
        combined = node.get("oneOf")
    if isinstance(combined, list):
        return union([ts_type(item, names, indent) for item in combined])
    kind = node.get("type")
    if isinstance(kind, list):
        return union([VALUE_TYPES.get(item, "unknown") for item in kind])
    if kind == "array":
        items = node.get("items")
        if isinstance(items, list):
            return "Array<" + union([ts_type(item, names, indent) for item in items]) + ">"
        return "Array<" + ts_type(items, names, indent) + ">"
    if kind == "object" or "properties" in node or "additionalProperties" in node:
        return object_type(node, names, indent)
    if isinstance(kind, str):
        return VALUE_TYPES.get(kind, "unknown")
    return "unknown"


def is_interface(schema: dict[str, Any]) -> bool:
    if not isinstance(schema.get("properties"), dict) or not schema["properties"]:
        return False
    if any(key in schema for key in ("oneOf", "anyOf", "allOf", "$ref", "enum", "const")):
        return False
    if isinstance(schema.get("type"), list):
        return False
    if not (schema.get("type") in (None, "object")):
        return False
    return not isinstance(schema.get("additionalProperties"), dict)


def declaration(name: str, schema: dict[str, Any], names: dict[str, str]) -> str:
    if is_interface(schema):
        properties = schema["properties"]
        required = set(schema.get("required", []))
        lines = [
            f"  {key if IDENTIFIER.match(key) else json.dumps(key)}"
            f"{'' if key in required else '?'}: {ts_type(prop if isinstance(prop, dict) else {}, names, 2)};"
            for key, prop in properties.items()
        ]
        return f"export interface {name} {{\n" + "\n".join(lines) + "\n}"
    return f"export type {name} = {ts_type(schema, names)};"


def pascal_case(stem: str) -> str:
    return "".join(part[:1].upper() + part[1:] for part in re.split(r"[-_]+", stem) if part)


def collect() -> tuple[list[pathlib.Path], dict[str, str]]:
    """Assign global TS names and return rendered declarations."""
    files = sorted(SCHEMA_DIR.glob("*.json"))
    if not files:
        raise SchemaError(f"no schema files in {SCHEMA_DIR}")

    names: dict[str, dict[str, str]] = {}  # file stem -> original name -> TS name
    assigned: dict[str, dict[str, Any]] = {}  # TS name -> schema (first body wins)
    sources: dict[str, str] = {}  # TS name -> defining file stem

    def reserve(file_stem: str, original: str, schema: dict[str, Any]) -> str:
        wanted = sanitize(original)
        body = body_key(schema)
        if wanted in assigned:
            if body_key(assigned[wanted]) == body:
                return wanted
            candidate = sanitize(pascal_case(file_stem) + wanted)
            counter = 2
            while candidate in assigned and body_key(assigned[candidate]) != body:
                candidate = sanitize(pascal_case(file_stem) + wanted + str(counter))
                counter += 1
            wanted = candidate
        assigned[wanted] = schema
        sources[wanted] = file_stem
        return wanted

    for path in files:
        try:
            schema = json.loads(path.read_text())
        except json.JSONDecodeError as error:
            raise SchemaError(f"{path.name}: invalid JSON: {error}") from error
        if not isinstance(schema, dict):
            raise SchemaError(f"{path.name}: schema root must be an object")
        stem = path.stem
        local: dict[str, str] = {}
        root_schema = {
            key: value for key, value in schema.items() if key not in ("$schema", "definitions")
        }
        reserve(stem, schema.get("title") or pascal_case(stem), root_schema)
        definitions = schema.get("definitions", schema.get("$defs", {}))
        if not isinstance(definitions, dict):
            raise SchemaError(f"{path.name}: definitions must be an object")
        for original in sorted(definitions):
            definition = definitions[original]
            if not isinstance(definition, dict):
                raise SchemaError(f"{path.name}: definition {original} must be an object")
            local[original] = reserve(stem, original, definition)
        # Refs are file-local and resolve to the globally assigned names.
        names[stem] = local

    rendered: dict[str, str] = {}
    for ts_name in sorted(assigned):
        stem = sources[ts_name]
        rendered[ts_name] = declaration(ts_name, assigned[ts_name], names[stem])
    return files, rendered


def generate() -> str:
    files, rendered = collect()
    parts: list[str] = [
        "// @generated by scripts/schema-ts.py -- do not edit",
        "// Source of truth: Rust contract crates (schemars) -> shared/contracts/schema/*.json",
        "",
        "// --- Types (generated from JSON Schema roots + definitions) ---",
        "",
    ]
    parts.append("\n\n".join(rendered[name] for name in sorted(rendered)))
    parts.append("")
    parts.append("// --- Embedded JSON Schemas (runtime Ajv validation, C10/T22) ---")
    parts.append("")
    schema_consts: list[str] = []
    schema_keys: list[str] = []
    for path in files:
        schema = json.loads(path.read_text())
        key = path.stem.replace("-", "_")
        schema_consts.append(
            f"export const {key}Schema = "
            f"{json.dumps(schema, ensure_ascii=False, sort_keys=True, separators=(',', ':'))} as const;"
        )
        schema_keys.append(key)
    parts.append("\n\n".join(schema_consts))
    parts.append("")
    parts.append("/** JSON Schemas keyed by contract name (schema file, `-` -> `_`). */")
    parts.append("export const SCHEMAS = {")
    for key in schema_keys:
        parts.append(f"  {key}: {key}Schema,")
    parts.append("} as const;")
    parts.append("")
    parts.append("export type SchemaKey = keyof typeof SCHEMAS;")
    parts.append("")
    return "\n".join(parts)


def main() -> int:
    try:
        output = generate()
    except SchemaError as error:
        print(f"schema-ts: {error}", file=sys.stderr)
        return 1
    if "--check" in sys.argv:
        current = OUT.read_text() if OUT.exists() else ""
        if current != output:
            print("schema-ts: out of date; run scripts/schema-ts.py", file=sys.stderr)
            return 1
        print("schema-ts: up to date")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(output)
    count = len(list(SCHEMA_DIR.glob("*.json")))
    print(f"schema-ts: wrote {OUT.relative_to(ROOT)} ({count} schemas)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
