/**
 * Shared settings field primitives (shell + `ui.settings.pane` contributions).
 *
 * A pane only renders fields for the shell-owned draft (`SettingsPaneProps`);
 * labels/help text are i18n keys resolved with the shell's translator. Keeping
 * the markup here avoids a second copy of input classes per pane (C17).
 */
import type { ReactNode } from "react";
import type { SettingsFieldError, SettingsPaneProps } from "./types";

/** The translator injected by the shell (same signature as `UiActions`/`UiValue.t`). */
type Translate = SettingsPaneProps["t"];

export const labelClass = "text-[12px] font-medium text-muted-foreground";
export const inputClass =
  "w-full rounded-control border border-border bg-background px-2.5 py-2 text-[13px] text-foreground placeholder:text-muted-foreground disabled:opacity-55";

/** Inline field error (i18n key → translated message). */
export function FieldError(props: { error: SettingsFieldError; t: Translate }): ReactNode {
  if (props.error === null) {
    return null;
  }
  return (
    <span className="text-[12px] text-destructive" role="alert">
      {props.t(props.error)}
    </span>
  );
}

export interface NumericFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  t: Translate;
  error: SettingsFieldError;
  step?: string;
  min?: number;
  max?: number;
}

/** Number input bound to a string draft field (empty = provider default). */
export function NumericField(props: NumericFieldProps): ReactNode {
  return (
    <label className="flex flex-col gap-1" htmlFor={props.id}>
      <span className={labelClass}>{props.label}</span>
      <input
        id={props.id}
        className={inputClass}
        type="number"
        inputMode="decimal"
        value={props.value}
        step={props.step ?? "1"}
        min={props.min}
        max={props.max}
        onChange={(event) => {
          props.onChange(event.target.value);
        }}
        aria-invalid={props.error !== null}
        aria-label={props.label}
      />
      <FieldError error={props.error} t={props.t} />
    </label>
  );
}

export interface TextFieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  t: Translate;
  error?: SettingsFieldError;
  hint?: string;
  type?: "text" | "url" | "password";
  placeholder?: string;
  span2?: boolean;
  disabled?: boolean;
}

/** Text/url/password input bound to a string draft field. */
export function TextField(props: TextFieldProps): ReactNode {
  const error = props.error ?? null;
  return (
    <label
      className={`flex flex-col gap-1${props.span2 === true ? " sm:col-span-2" : ""}`}
      htmlFor={props.id}
    >
      <span className={labelClass}>{props.label}</span>
      <input
        id={props.id}
        className={inputClass}
        type={props.type ?? "text"}
        inputMode={props.type === "url" ? "url" : undefined}
        autoComplete={props.type === "password" ? "off" : undefined}
        spellCheck={props.type === "password" ? false : undefined}
        value={props.value}
        disabled={props.disabled === true}
        placeholder={props.placeholder}
        onChange={(event) => {
          props.onChange(event.target.value);
        }}
        aria-invalid={error !== null}
        aria-label={props.label}
      />
      <FieldError error={error} t={props.t} />
      {props.hint === undefined ? null : (
        <span className="text-[12px] text-muted-foreground">{props.hint}</span>
      )}
    </label>
  );
}

/** Section legend + fields grid shared by pane sections. */
export function PaneSection(props: { title: string; children: ReactNode }): ReactNode {
  return (
    <fieldset className="rounded-document bg-muted p-4">
      <legend className="px-1.5 text-[12px] font-medium text-muted-foreground">
        {props.title}
      </legend>
      <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2">{props.children}</div>
    </fieldset>
  );
}
