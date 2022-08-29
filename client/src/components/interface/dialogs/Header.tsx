import * as React from 'react'
import { DialogTitle, IconButton } from '@mui/material'
import { Clear } from '@mui/icons-material'

interface Props {
  title: string
  action: () => void
}

export default function DialogHeader({ title, action }: Props) {
  return (
    <DialogTitle>
      {title}
      <IconButton
        onClick={action}
        style={{ position: 'absolute', right: 5, top: 5 }}
      >
        <Clear />
      </IconButton>
    </DialogTitle>
  )
}
