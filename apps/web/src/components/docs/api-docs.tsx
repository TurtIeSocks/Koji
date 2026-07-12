import { ApiReferenceReact } from '@scalar/api-reference-react'
import '@scalar/api-reference-react/style.css'

/**
 * Full-page Scalar API reference for the `/api/v2` surface.
 *
 * Fed by the server's code-first OpenAPI document (utoipa `ApiDoc`), served at
 * `GET /api/v2/openapi.yaml` — same origin, so a relative URL suffices. The
 * endpoint emits the doc as JSON despite the `.yaml` path; Scalar sniffs the
 * format from the body, so the extension mismatch is harmless. The doc is
 * unauthenticated, but this page is gated behind `<Authenticated>` in `App`.
 */
export function ApiDocs() {
  return <ApiReferenceReact configuration={{ url: '/api/v2/openapi.yaml' }} />
}
