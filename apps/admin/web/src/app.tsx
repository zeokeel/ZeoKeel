import { AdminShellSlot, ADMIN_PAGES } from "./generated/registry";
import type { AdminPage } from "@zeokeel/admin-shell";

export function AdminApp() {
  return <AdminShellSlot renderPage={(page: AdminPage) => {
    const Page = ADMIN_PAGES[page];
    return <Page onError={(message: string) => console.error(message)} />;
  }} />;
}
