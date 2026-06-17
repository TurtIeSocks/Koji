import * as React from 'react'
import Alert, { type AlertProps } from '@mui/material/Alert'
import Collapse, { type CollapseProps } from '@mui/material/Collapse'
import IconButton, { type IconButtonProps } from '@mui/material/IconButton'
import Stack, { type StackProps } from '@mui/material/Stack'
import CloseIcon from '@mui/icons-material/Close'
import AlertTitle, { type AlertTitleProps } from '@mui/material/AlertTitle'

import { useAlertTimer } from '@hooks/useAlertTimer'
import { useStatic } from '@hooks/useStatic'

interface Props {
  CollapseProps?: CollapseProps
  StackProps?: StackProps
  AlertProps?: AlertProps
  IconButtonProps?: IconButtonProps
  AlertTitleProps?: AlertTitleProps
  children: React.ReactNode
  title?: string
}

export default function Notification({
  CollapseProps,
  StackProps,
  AlertProps,
  IconButtonProps,
  AlertTitleProps,
  children,
  title,
}: Props) {
  const setHover = useAlertTimer()

  return (
    <Collapse
      sx={{
        position: 'absolute',
        bottom: 0,
        width: '66%',
        mx: 'auto',
        left: 0,
        right: 0,
        transition: '0.50s ease-in-out',
      }}
      {...CollapseProps}
    >
      <Stack
        sx={{ width: '100%', maxHeight: '90vh' }}
        spacing={2}
        {...StackProps}
      >
        <Alert
          action={
            <IconButton
              aria-label="close"
              color="inherit"
              size="small"
              onClick={() =>
                useStatic.setState((prev) => ({
                  notification: {
                    ...prev.notification,
                    message: '',
                    status: 0,
                  },
                }))
              }
              {...IconButtonProps}
            >
              <CloseIcon fontSize="inherit" />
            </IconButton>
          }
          onMouseEnter={() => setHover(true)}
          onMouseLeave={() => setHover(false)}
          {...AlertProps}
          sx={{ mb: 2, zIndex: 10000, ...AlertProps?.sx }}
        >
          <AlertTitle {...AlertTitleProps}>
            <strong>{title}</strong>
          </AlertTitle>
          {children}
        </Alert>
      </Stack>
    </Collapse>
  )
}
