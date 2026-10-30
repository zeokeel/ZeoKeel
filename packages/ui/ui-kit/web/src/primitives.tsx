import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";

type ButtonVariant = "default" | "primary" | "destructive" | "ghost";
type ButtonSize = "default" | "sm";

function buttonClasses(variant: ButtonVariant, size: ButtonSize, className: string): string {
  const base =
    "inline-flex items-center justify-center gap-1.5 rounded-control font-medium transition-colors disabled:pointer-events-none disabled:opacity-40";
  const sizeClass = size === "sm" ? "px-2 py-1 text-[12px]" : "px-3 py-1.5 text-[13px]";
  let variantClass = "border border-border bg-background text-foreground hover:bg-muted";
  if (variant === "primary") {
    variantClass = "bg-foreground text-background hover:bg-foreground/90";
  } else if (variant === "destructive") {
    variantClass = "border border-destructive/40 text-destructive hover:bg-destructive/10";
  } else if (variant === "ghost") {
    variantClass = "text-muted-foreground hover:bg-foreground/[0.05] hover:text-foreground";
  }
  return [base, sizeClass, variantClass, className].filter((part) => part !== "").join(" ");
}

export function Button(
  props: ButtonHTMLAttributes<HTMLButtonElement> & {
    variant?: ButtonVariant;
    size?: ButtonSize;
  },
) {
  const { variant = "default", size = "default", className = "", ...rest } = props;
  return <button {...rest} className={buttonClasses(variant, size, className)} />;
}

export function Switch(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className = "", ...rest } = props;
  return <input {...rest} type="checkbox" className={`zk-switch ${className}`.trim()} />;
}

export function StatusDot(props: { live?: boolean; className?: string }) {
  const { live = false, className = "" } = props;
  return (
    <span
      aria-hidden="true"
      className={`h-1.5 w-1.5 shrink-0 rounded-capsule ${
        live ? "zk-pulse-dot bg-live" : "bg-muted-foreground"
      } ${className}`.trim()}
    />
  );
}

export function Chip(props: { tone?: "neutral" | "destructive"; children: ReactNode }) {
  const toneClass =
    props.tone === "destructive"
      ? "border-destructive/40 text-destructive"
      : "border-border text-muted-foreground";
  return (
    <span className={`inline-flex items-center rounded-sm border px-1.5 py-0.5 text-[12px] ${toneClass}`}>
      {props.children}
    </span>
  );
}
