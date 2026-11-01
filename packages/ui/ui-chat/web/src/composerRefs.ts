/**
 * Composer reference/attachment helpers (`2026-11-01-client-composer-surface`).
 *
 * Pure functions shared by the composer and its tests:
 * - `/` and `@` trigger detection at the caret (word-start only, IME-safe by the caller);
 * - the normative user-message reference block (`composeOutgoingText` /
 *   `parseUserContent`, the single implementation of the format);
 * - client attachment limits + workspace-path validation.
 *
 * The block is plain text appended to the user body (`chat.send` is unchanged);
 * the model reads the referenced workspace files with its existing file tools.
 */
import type {
  AgentUsage,
  ChatMessage,
  CommandDescriptor,
  DraftRef,
  SkillCandidate,
} from "@zeokeel/ui-kit";

/** Exact reference-block marker; must stay in sync with `composeOutgoingText`. */
export const REFERENCE_BLOCK_MARKER = "--- referenced files ---";

/** Exact `/<skill>` Instructions block marker (before the trailing references). */
export const SKILL_INSTRUCTIONS_MARKER = "--- skill instructions ---";

/** One skill Instructions block is ≤8 KiB (the host also enforces `skill/too-large`). */
export const SKILL_INSTRUCTIONS_MAX_BYTES = 8 * 1024;

/** One attachment must be ≤1 MiB of raw bytes (serve `/ipc` body limit stays 2 MiB). */
export const ATTACHMENT_MAX_BYTES = 1024 * 1024;
/** At most 5 references + attachments per message (R2/R3). */
export const ATTACHMENT_MAX_COUNT = 5;
/** Host/client attachment extension allow-list (8 kinds; design §2.2). */
export const ATTACHMENT_EXTENSIONS: readonly string[] = [
  "png",
  "jpg",
  "jpeg",
  "webp",
  "gif",
  "pdf",
  "txt",
  "md",
  "json",
  "csv",
  "log",
];
const IMAGE_EXTENSIONS: readonly string[] = ["png", "jpg", "jpeg", "webp", "gif"];

/** Lowercase extension of a file name (`null` when there is none). */
export function attachmentExtensionOf(name: string): string | null {
  const index = name.lastIndexOf(".");
  if (index <= 0 || index === name.length - 1) {
    return null;
  }
  return name.slice(index + 1).toLowerCase();
}

/** Whether the extension is in the attachment allow-list. */
export function isAttachmentAllowed(name: string): boolean {
  const extension = attachmentExtensionOf(name);
  return extension !== null && ATTACHMENT_EXTENSIONS.includes(extension);
}

/** Whether the file name has an image extension (thumbnail candidate). */
export function isImageAttachment(name: string): boolean {
  const extension = attachmentExtensionOf(name);
  return extension !== null && IMAGE_EXTENSIONS.includes(extension);
}

/** Compact byte-size label (`512 B`, `1.5 KiB`); unit tokens are not localized copy. */
export function attachmentSizeLabel(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) {
    return "0 B";
  }
  if (bytes < 1024) {
    return `${String(Math.round(bytes))} B`;
  }
  return `${(bytes / 1024).toFixed(1)} KiB`;
}

function hasControlCharacters(value: string): boolean {
  for (const character of value) {
    const code = character.codePointAt(0);
    if (code !== undefined && (code < 0x20 || code === 0x7f)) {
      return true;
    }
  }
  return false;
}

/**
 * Validate one workspace-relative reference path.
 *
 * Rejects empty, control characters/newlines, absolute paths, empty/`.`/`..`
 * segments; directory references may keep one trailing `/`.
 */
export function isValidReferencePath(path: string): boolean {
  if (path === "" || hasControlCharacters(path)) {
    return false;
  }
  if (path.startsWith("/") || path.startsWith("\\")) {
    return false;
  }
  const withoutTrailingSlash = path.endsWith("/") ? path.slice(0, -1) : path;
  if (withoutTrailingSlash === "") {
    return false;
  }
  const segments = withoutTrailingSlash.split("/");
  return !segments.some((segment) => segment === "" || segment === "." || segment === "..");
}

/** Stable key used to de-duplicate block lines. */
function referenceKey(path: string): string {
  return path.endsWith("/") ? path.slice(0, -1) : path;
}

/**
 * Encode one reference path for the block: one `@` prefix, then the path
 * verbatim (`@`-leading paths therefore appear as `@@path` and decode back to
 * exactly one leading `@`).
 */
function encodeReferencePath(path: string): string {
  return `@${path}`;
}

/** UTF-8 byte length (the Instructions cap is byte-based, not UTF-16 units). */
export function utf8ByteLength(value: string): number {
  return new TextEncoder().encode(value).length;
}

function truncateUtf8(value: string, maxBytes: number): string {
  if (utf8ByteLength(value) <= maxBytes) {
    return value;
  }
  let bytes = 0;
  let out = "";
  for (const character of value) {
    const size = utf8ByteLength(character);
    if (bytes + size > maxBytes) {
      break;
    }
    bytes += size;
    out += character;
  }
  return out;
}

/** Strip control characters (newline/tab allowed) from a skill body. */
function sanitizeSkillContent(content: string): string {
  let out = "";
  for (const character of content) {
    const code = character.codePointAt(0);
    if (code === undefined) {
      continue;
    }
    const control = code < 0x20 || code === 0x7f;
    if (control && character !== "\n" && character !== "\t") {
      continue;
    }
    out += character;
  }
  return out;
}

/** Whether a skill name can be written into the `skill:` header line safely. */
function isValidSkillName(name: string): boolean {
  return name.trim() !== "" && name === name.trim() && !hasControlCharacters(name);
}

/** The one Instructions chip of a draft (kind `skill`), if any. */
function skillRefOf(refs: readonly DraftRef[]): DraftRef | undefined {
  return refs.find((ref) => ref.kind === "skill");
}

/**
 * Compose the outgoing user text: body + optional skill Instructions block +
 * normative trailing reference block.
 *
 * - ready workspace references and ready attachments only (uploading/error chips
 *   are never sent; no half-baked attachments);
 * - invalid paths and duplicates are skipped (compose never emits a fake entry);
 * - at most one Instructions chip; its body is trimmed/sanitized and capped at
 *   8 KiB (the host already rejected larger content via `skill/too-large`);
 * - no references → the body (plus Instructions, when present) is returned.
 */
export function composeOutgoingText(body: string, refs: readonly DraftRef[]): string {
  const seen = new Set<string>();
  const lines: string[] = [];
  for (const ref of refs) {
    if (ref.kind === "skill") {
      continue;
    }
    if (ref.kind === "attachment" && ref.status !== "ready") {
      continue;
    }
    if (!isValidReferencePath(ref.path)) {
      continue;
    }
    const key = referenceKey(ref.path);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    lines.push(encodeReferencePath(ref.path));
  }
  const skill = skillRefOf(refs);
  let text = body;
  if (skill !== undefined) {
    const block = skillInstructionBlock(skill);
    if (block !== null) {
      const trimmed = text.replace(/\s+$/, "");
      text = trimmed === "" ? block : `${trimmed}\n\n${block}`;
    }
  }
  if (lines.length === 0) {
    return text;
  }
  const trimmed = text.replace(/\s+$/, "");
  const block = `${REFERENCE_BLOCK_MARKER}\n${lines.join("\n")}`;
  return trimmed === "" ? block : `${trimmed}\n\n${block}`;
}

function skillInstructionBlock(skill: DraftRef): string | null {
  const name = skill.name ?? skill.path;
  if (!isValidSkillName(name) || skill.content === undefined) {
    return null;
  }
  const content = truncateUtf8(
    sanitizeSkillContent(skill.content).replace(/\s+$/, ""),
    SKILL_INSTRUCTIONS_MAX_BYTES,
  );
  if (content === "") {
    return null;
  }
  const header = [`skill: ${name}`, `source: ${skill.source ?? "workspace"}`];
  if (skill.version !== undefined && skill.version !== null && skill.version !== "") {
    header.push(`version: ${skill.version}`);
  }
  return `${SKILL_INSTRUCTIONS_MARKER}\n${header.join("\n")}\n\n${content}`;
}

/** Restored `/<skill>` Instructions block of a persisted user message. */
export interface ParsedSkillInstruction {
  /** Skill name from the block header. */
  name: string;
  /** Instructions body (trimmed). */
  content: string;
}

export interface ParsedUserContent {
  /** User-visible body (Instructions/reference blocks removed). */
  body: string;
  /** References restored from the block (workspace kind; dir refs keep the `/`). */
  refs: DraftRef[];
  /** Restored skill Instructions block (`null` when absent/malformed). */
  instruction: ParsedSkillInstruction | null;
}

/**
 * Split a persisted user message into body + Instructions + references.
 *
 * Both blocks only match in their canonical trailing positions (start of the
 * message or after a blank line). Anything else is ordinary body text (no false
 * chips); invalid/duplicate reference lines are skipped inside a genuine block.
 * The Instructions block (when present) precedes the reference block.
 */
export function parseUserContent(text: string): ParsedUserContent {
  const stripped = text.replace(/\s+$/, "");
  const references = splitReferenceBlock(stripped);
  const skill = splitSkillBlock(references.rest);
  if (references.refs.length === 0 && skill.instruction === null) {
    return { body: text, refs: [], instruction: null };
  }
  return {
    body: skill.rest,
    refs: references.refs,
    instruction: skill.instruction,
  };
}

function splitReferenceBlock(text: string): { rest: string; refs: DraftRef[] } {
  let blockStart = -1;
  if (text.startsWith(`${REFERENCE_BLOCK_MARKER}\n`)) {
    blockStart = 0;
  } else {
    const index = text.lastIndexOf(`\n\n${REFERENCE_BLOCK_MARKER}\n`);
    if (index >= 0) {
      blockStart = index;
    }
  }
  if (blockStart < 0) {
    return { rest: text, refs: [] };
  }
  const rest = blockStart === 0 ? "" : text.slice(0, blockStart);
  const block = text.slice(blockStart === 0 ? 0 : blockStart + 2);
  const [markerLine, ...lines] = block.split("\n");
  if (markerLine !== REFERENCE_BLOCK_MARKER) {
    return { rest: text, refs: [] };
  }
  const refs: DraftRef[] = [];
  const seen = new Set<string>();
  for (const line of lines) {
    if (line === "") {
      continue;
    }
    if (!line.startsWith("@")) {
      // The marker is part of the body, not a trailing block.
      return { rest: text, refs: [] };
    }
    const path = line.slice(1);
    if (!isValidReferencePath(path)) {
      continue;
    }
    const key = referenceKey(path);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    const name = referenceKey(path).split("/").pop() ?? path;
    refs.push({ kind: "workspace", path, name });
  }
  if (refs.length === 0) {
    return { rest: text, refs: [] };
  }
  return { rest, refs };
}

function splitSkillBlock(text: string): {
  rest: string;
  instruction: ParsedSkillInstruction | null;
} {
  let blockStart = -1;
  if (text.startsWith(`${SKILL_INSTRUCTIONS_MARKER}\n`)) {
    blockStart = 0;
  } else {
    const index = text.lastIndexOf(`\n\n${SKILL_INSTRUCTIONS_MARKER}\n`);
    if (index >= 0) {
      blockStart = index;
    }
  }
  if (blockStart < 0) {
    return { rest: text, instruction: null };
  }
  const rest = blockStart === 0 ? "" : text.slice(0, blockStart);
  const block = text.slice(blockStart === 0 ? 0 : blockStart + 2);
  const [markerLine, ...lines] = block.split("\n");
  if (markerLine !== SKILL_INSTRUCTIONS_MARKER) {
    return { rest: text, instruction: null };
  }
  let name = "";
  let headerEnd = -1;
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index] ?? "";
    if (line === "") {
      headerEnd = index;
      break;
    }
    if (line.startsWith("skill:")) {
      name = line.slice("skill:".length).trim();
    }
  }
  if (headerEnd < 0 || !isValidSkillName(name)) {
    return { rest: text, instruction: null };
  }
  const content = lines.slice(headerEnd + 1).join("\n").replace(/\s+$/, "");
  return { rest, instruction: { name, content } };
}

export interface ComposerTrigger {
  kind: "command" | "file";
  /** Text typed after the trigger character (filter query). */
  query: string;
  /** Index of the trigger character in the textarea value. */
  start: number;
  /** Caret index (end of the query). */
  end: number;
}

/**
 * Detect a word-start `/` or `@` trigger immediately before the caret.
 *
 * Only word start counts (`^` or whitespace before the trigger); mid-word
 * characters stay ordinary text. IME composing is handled by the caller (no
 * trigger while composing).
 */
export function detectComposerTrigger(text: string, caret: number): ComposerTrigger | null {
  const end = Math.max(0, Math.min(caret, text.length));
  let start = end;
  while (start > 0) {
    const character = text.charAt(start - 1);
    if (character === "/" || character === "@") {
      break;
    }
    if (/\s/.test(character)) {
      return null;
    }
    start -= 1;
  }
  if (start === 0) {
    return null;
  }
  const trigger = text.charAt(start - 1);
  if (trigger !== "/" && trigger !== "@") {
    return null;
  }
  if (start - 1 > 0) {
    const before = text.charAt(start - 2);
    if (!/\s/.test(before)) {
      return null;
    }
  }
  const query = text.slice(start, end);
  if (/\s/.test(query)) {
    return null;
  }
  return { kind: trigger === "/" ? "command" : "file", query, start: start - 1, end };
}

/** Remove the trigger text (`/@query`) and report the new caret position. */
export function removeComposerTrigger(
  text: string,
  trigger: ComposerTrigger,
): { text: string; caret: number } {
  return {
    text: `${text.slice(0, trigger.start)}${text.slice(trigger.end)}`,
    caret: trigger.start,
  };
}

/** Case-insensitive command filter over id/title/group/hint. */
export function filterCommands(
  commands: readonly CommandDescriptor[],
  query: string,
): CommandDescriptor[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") {
    return [...commands];
  }
  return commands.filter((command) => {
    const haystack = `${command.id} ${command.title} ${command.group} ${command.hint ?? ""}`;
    return haystack.toLowerCase().includes(needle);
  });
}

/** Case-insensitive `/<skill>` candidate filter over name/description. */
export function filterSkillCandidates(
  skills: readonly SkillCandidate[],
  query: string,
): SkillCandidate[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") {
    return [...skills];
  }
  return skills.filter((skill) => {
    const haystack = `${skill.name} ${skill.description}`;
    return haystack.toLowerCase().includes(needle);
  });
}

const ZERO_USAGE: AgentUsage = {
  input_tokens: 0,
  output_tokens: 0,
  total_tokens: 0,
  cached_tokens: 0,
  reasoning_tokens: 0,
  cost_micros: null,
};

export interface ComposerUsage {
  /** Usage of the most recent assistant turn (streaming updates in place). */
  turn: AgentUsage | null;
  /** Sum of all persisted assistant usage in the conversation. */
  session: AgentUsage | null;
}

/**
 * Usage for the composer footer from the existing `usage` event / persisted
 * `AgentUsage` only (D4: no extra provider calls, no context-window estimates).
 */
export function composerUsage(messages: readonly ChatMessage[]): ComposerUsage {
  let turn: AgentUsage | null = null;
  const session: AgentUsage = { ...ZERO_USAGE };
  for (const message of messages) {
    if (message.role !== "assistant" || message.usage === undefined || message.usage === null) {
      continue;
    }
    const usage = message.usage;
    turn = usage;
    session.input_tokens += usage.input_tokens;
    session.output_tokens += usage.output_tokens;
    session.total_tokens += usage.total_tokens;
    session.cached_tokens += usage.cached_tokens;
    session.reasoning_tokens += usage.reasoning_tokens;
  }
  return { turn, session: turn === null ? null : session };
}

/** Client-side attachment validation; returns an i18n error key or `null`. */
export function validateAttachment(
  file: File,
  currentCount: number,
): "attachmentLimit" | "attachmentEmpty" | "attachmentTooLarge" | "attachmentTypeNotAllowed" | null {
  if (currentCount >= ATTACHMENT_MAX_COUNT) {
    return "attachmentLimit";
  }
  if (file.size === 0) {
    return "attachmentEmpty";
  }
  if (file.size > ATTACHMENT_MAX_BYTES) {
    return "attachmentTooLarge";
  }
  if (!isAttachmentAllowed(file.name)) {
    return "attachmentTypeNotAllowed";
  }
  return null;
}
