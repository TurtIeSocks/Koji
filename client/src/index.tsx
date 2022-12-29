import * as React from 'react'
import { createRoot } from 'react-dom/client'

import '@assets/index.scss'

import ErrorBoundary from '@components/ErrorBoundary'
import App from './App'

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
)
