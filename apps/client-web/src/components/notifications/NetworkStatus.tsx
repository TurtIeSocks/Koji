import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import { Typography } from '@mui/material'

import Notification from './Base'

export default function NetworkAlert() {
  const { status, severity, message } = useStatic((s) => s.notification)

  return (
    <Notification
      CollapseProps={{ in: !!status }}
      AlertProps={{ severity }}
      title={`Network Status: ${status} ${
        {
          200: 'Success',
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
