import { AdminShell, type AdminPage } from "@zeokeel/admin-shell";
import { Users } from "@zeokeel/admin-users";
import { Wallets } from "@zeokeel/admin-wallet";
import { Pricing } from "@zeokeel/admin-pricing";
import { AuditLog } from "@zeokeel/admin-audit";
import { Usage } from "@zeokeel/admin-usage";
import { Jobs } from "@zeokeel/admin-jobs";
import { Settings } from "@zeokeel/admin-settings";

export function AdminApp() {
  return <AdminShell renderPage={(page: AdminPage) => {
    switch (page) {
      case "users": return <Users onError={console.error} />;
      case "wallet": return <Wallets onError={console.error} />;
      case "pricing": return <Pricing onError={console.error} />;
      case "audit": return <AuditLog onError={console.error} />;
      case "usage": return <Usage onError={console.error} />;
      case "jobs": return <Jobs onError={console.error} />;
      case "settings": return <Settings onError={console.error} />;
    }
  }} />;
}















