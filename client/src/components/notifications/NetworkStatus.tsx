import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import { Typography } from '@mui/material'

import Notification from './Base'

export default function NetworkAlert() {
  const { status, severity, message } = useStatic((s) => s.networkError)

  const [hover, setHover] = React.useState(false)

  React.useEffect(() => {
    if (status && !hover) {
      const timer = setTimeout(() => {
        useStatic.setState((prev) => ({
          networkError: { ...prev.networkError, message: '', status: 0 },
        }))
      }, 5000)
      return () => clearTimeout(timer)
    }
  }, [status, hover])

  return (
    <Notification
      CollapseProps={{
        in: !!status,
      }}
      AlertProps={{
        severity,
        onMouseEnter: () => setHover(true),
        onMouseLeave: () => setHover(false),
      }}
      IconButtonProps={{
        onClick: () =>
          useStatic.setState((prev) => ({
            networkError: { ...prev.networkError, message: '', status: 0 },
          })),
      }}
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
