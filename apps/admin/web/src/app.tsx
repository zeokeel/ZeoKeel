import { AdminShellSlot, ADMIN_PAGES } from "./generated/registry";
import type { AdminPage } from "@zeokeel/admin-shell";

// Non-enterprise baselines (personal) generate an empty admin registry; keep the
// shell renderable so the workspace still typechecks/builds in that baseline.
const NoopPage = () => null;

export function AdminApp() {
  return <AdminShellSlot renderPage={(page: AdminPage) => {
    const Page = ADMIN_PAGES[page] ?? NoopPage;
    return <Page onError={(message: string) => { console.error(message); }} />;
  }} />;
}
