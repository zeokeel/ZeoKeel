import { AdminShellSlot, ADMIN_PAGES } from "./generated/registry";
import type { AdminPage } from "@zeokeel/admin-shell";

// Non-enterprise baselines (personal) generate an empty admin registry; keep the
// shell renderable so the workspace still typechecks/builds in that baseline.
type AdminPageComponent = (props: { onError: (message: string) => void }) => React.ReactNode;

const NoopPage: AdminPageComponent = () => null;

// Widen to Partial for both composition baselines: personal emits an empty stub registry,
// enterprise binds all pages. Keeps the shell renderable without a per-edition code branch.
const pages: Partial<Record<AdminPage, AdminPageComponent>> = ADMIN_PAGES;

export function AdminApp() {
  return <AdminShellSlot renderPage={(page: AdminPage) => {
    const Page = pages[page] ?? NoopPage;
    return <Page onError={(message: string) => { console.error(message); }} />;
  }} />;
}
