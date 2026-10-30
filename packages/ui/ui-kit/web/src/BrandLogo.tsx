import { useUi } from "./context";

/**
 * Official channel logo from `zk compose` (`BRAND.logoLight/logoDark`).
 * Never hand-draw a mark here; this is the only logo component in the repo.
 */
export function BrandLogo(props: { className?: string; decorative?: boolean }) {
  const { brand, theme } = useUi();
  const src = theme === "dark" ? brand.logoDark : brand.logoLight;
  return (
    <img
      src={src}
      alt={props.decorative === true ? "" : brand.name}
      className={props.className}
      draggable={false}
    />
  );
}
