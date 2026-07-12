import { ApiReferenceReact } from '@scalar/api-reference-react'
import '@scalar/api-reference-react/style.css'
import { useTheme } from '@/hooks/use-theme'
import { resolveMode } from '@/lib/theme-context'

/**
 * Full-page Scalar API reference for the `/api/v2` surface.
 *
 * Fed by the server's code-first OpenAPI document (utoipa `ApiDoc`), served at
 * `GET /api/v2/openapi.yaml` — same origin, so a relative URL suffices. The
 * endpoint emits the doc as JSON despite the `.yaml` path; Scalar sniffs the
 * format from the body, so the extension mismatch is harmless. The doc is
 * unauthenticated, but this page is gated behind `<Authenticated>` in `App`.
 *
 * Scalar's light/dark is slaved to the app's theme: `forceDarkModeState` locks
 * it to the app's resolved mode, and its own toggle is hidden so the sidebar
 * theme switch is the single control. `useTheme` re-renders on toggle, so this
 * tracks reactively.
 */
export function ApiDocs() {
  const [theme] = useTheme()
  return (
    <ApiReferenceReact
      configuration={{
        url: '/api/v2/openapi.yaml',
        forceDarkModeState: resolveMode(theme),
        hideDarkModeToggle: true,
      }}
    />
  )
}
