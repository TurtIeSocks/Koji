import * as React from 'react'
import { DialogTitle, IconButton } from '@mui/material'
import { Clear } from '@mui/icons-material'

interface Props {
  children?: React.ReactNode
  action: () => void
}

export default function DialogHeader({ children, action }: Props) {
  return (
    <DialogTitle>
      {children}
      <IconButton
        onClick={action}
        style={{ position: 'absolute', right: 5, top: 5 }}
      >
        <Clear />
      </IconButton>
    </DialogTitle>
  )
}
