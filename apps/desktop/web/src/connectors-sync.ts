/**
 * Connector catalog → host IPC seams (config spec R6/R7).
 *
 * The web layer only transports the raw catalog; validation/installation stays
 * in the Rust host (`zk_connector_mcp`), so there is no second catalog parser
 * here (C17).
 */
import type { Transport } from "./types";
import type { ConnectorCatalog } from "./store";

/** Install/refresh the catalog; rejects so callers can surface the error. */
export async function syncConnectorCatalog(
  transport: Transport,
  catalog: ConnectorCatalog,
): Promise<void> {
  await transport.call("connectors.configure", { catalog: catalog.catalog });
}

/** Drop cached MCP sessions so the next tool call re-initializes from config. */
export async function reloadConnectorSessions(transport: Transport): Promise<void> {
  await transport.call("connectors.reload", {});
}

/** Clear catalog-provided config (logout/disconnect). */
export async function clearConnectorCatalog(transport: Transport): Promise<void> {
  await transport.call("connectors.clear", {});
}
