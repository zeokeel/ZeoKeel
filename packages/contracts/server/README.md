# zk-contract-server

M2 server domain marker contract: `ServerModule { id, version }` and the `srv.*` role seam.
Packages depend on this crate only (C4); they never import another server implementation.

Admin console HTTP wire responses (`Admin*Response` / `Admin*Item`) mirror the
`/api/admin/*` bodies emitted by `srv.*` routes. The admin SPA validates every
response against the generated JSON Schemas (`shared/contracts/schema/admin-*-response.json`
via `scripts/schema-ts.py` + `@zeokeel/admin-ui-kit` Ajv decoders, C10/T22) before use.
