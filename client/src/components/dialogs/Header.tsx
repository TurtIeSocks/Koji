import * as React from 'react'
import { DialogTitle, DialogTitleProps, IconButton } from '@mui/material'
import Clear from '@mui/icons-material/Clear'

interface Props extends DialogTitleProps {
  children?: React.ReactNode
  action?: () => void
}

export default function DialogHeader({ children, action, ...rest }: Props) {
  return (
    <DialogTitle
      sx={(theme) => ({
        bgcolor:
          theme.palette.mode === 'dark'
            ? theme.palette.grey[700]
            : theme.palette.grey[200],
      })}
      {...rest}
    >
      {children}
      {!!action && (
        <IconButton
          onClick={action}
          style={{ position: 'absolute', right: 12, top: 12 }}
        >
          <Clear />
        </IconButton>
      )}
    </DialogTitle>
  )
}
