import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import { Typography } from '@mui/material'

import Notification from './Base'

export default function NetworkAlert() {
  const { status, severity, message } = useStatic((s) => s.networkError)

  return (
    <Notification
      CollapseProps={{
        in: !!status,
      }}
      AlertProps={{ severity }}
      IconButtonProps={{
        onClick: () =>
          useStatic.setState((prev) => ({
            networkError: { ...prev.networkError, message: '', status: 0 },
          })),
      }}
      title={`Network Status: ${status} ${
        {
          400: 'Bad Request',
          401: 'Unauthorized',
          404: 'Not Found',
          408: 'Network Timeout',
          413: 'Payload Too Large',
          500: 'Server Error',
        }[status] || 'Unknown Error'
      }`}
    >
      <Typography>{message}</Typography>
    </Notification>
  )
}
