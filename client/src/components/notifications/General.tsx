import * as React from 'react'
import { useStatic } from '@hooks/useStatic'
import { Typography } from '@mui/material'

import Notification from './Base'

export default function GeneralAlert() {
  const { status, severity, message } = useStatic((s) => s.notification)

  return (
    <Notification
      CollapseProps={{ in: !!status }}
      AlertProps={{ severity }}
      title={
        {
          error: 'Error:',
          warning: 'Warning:',
          info: 'Info:',
          success: 'Success:',
        }[severity || 'info']
      }
    >
      <Typography>{message}</Typography>
    </Notification>
  )
}
