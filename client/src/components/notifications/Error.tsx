import * as React from 'react'
import { useStatic } from '@hooks/useStatic'

import Notification from './Base'

export default function NetworkAlert() {
  const { status, severity, message } = useStatic((s) => s.networkError)

  return (
    <Notification
      CollapseProps={{
        in: !!message,
      }}
      AlertProps={{ severity }}
      IconButtonProps={{
        sx: { color: 'white' },
        onClick: () =>
          useStatic.setState((prev) => ({
            networkError: { ...prev.networkError, message: '' },
          })),
      }}
      title={`Network Error: ${status}`}
    >
      {message}
    </Notification>
  )
}
