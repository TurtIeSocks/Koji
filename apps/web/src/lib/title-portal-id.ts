/**
 * DOM id of the title portal target.
 *
 * Used by `<Title>` to locate the destination element via `createPortal`.
 * Kept stable for backward compatibility with the previous `id="breadcrumb"`
 * slot in the default Layout header — this constant is the canonical one used
 * by the new `<AppBar>` / `<Title>` pair.
 */
const TITLE_PORTAL_ID = "react-admin-title";

export { TITLE_PORTAL_ID };
