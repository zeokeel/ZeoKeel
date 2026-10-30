import { BRAND } from "../generated/brand";
import { useThemeResolution } from "../theme";

/**
 * Official channel logo, injected by `zk compose` from `brands/<channel>/`
 * (`src/generated/brand.ts` + `public/brand/*.svg`). The repo ships no other
 * logo artifact; do not hand-draw a mark here.
 */
export function BrandLogo(props: { className?: string; decorative?: boolean }) {
  const theme = useThemeResolution();
  const src = theme === "dark" ? BRAND.logoDark : BRAND.logoLight;
  return (
    <img
      src={src}
      alt={props.decorative === true ? "" : BRAND.name}
      className={props.className}
      draggable={false}
    />
  );
}
