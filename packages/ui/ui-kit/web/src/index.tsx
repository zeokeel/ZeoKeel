/**
 * @zeokeel/ui-kit — shared DTOs, UiProvider/useUi, brand logo, icons, primitives.
 * Only depends on React; no app or sibling UI package imports.
 */
export * from "./types";
export * from "./context";
export * from "./BrandLogo";
export * from "./icons";
export * from "./primitives";
export * from "./guards";
export * from "./boundary";
export * from "./focus";

/** ui-kit is a non-slot library (`ui.kit` role); it contributes no components. */
export const components = {} as const;
