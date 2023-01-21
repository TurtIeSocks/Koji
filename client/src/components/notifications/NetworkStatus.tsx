import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import { Typography } from '@mui/material'

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
        onClick: () =>
          useStatic.setState((prev) => ({
            networkError: { ...prev.networkError, message: '' },
          })),
      }}
      title={`Network Status: ${status}`}
    >
      <Typography>{message}</Typography>
    </Notification>
  )
}
