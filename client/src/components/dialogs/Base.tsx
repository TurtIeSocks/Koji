import * as React from 'react'
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitleProps,
  type DialogProps,
  DialogContentProps,
  DialogActionsProps,
} from '@mui/material'
import DialogHeader from './Header'

interface Props {
  open: boolean
  onClose: () => void
  title: string
  children: React.ReactNode
  Components?: {
    Dialog?: Partial<DialogProps>
    DialogTitle?: Partial<DialogTitleProps>
    DialogContent?: Partial<DialogContentProps>
    DialogActions?: Partial<DialogActionsProps>
  }
}
export default function BaseDialog({
  open,
  onClose,
  title,
  children,
  Components,
}: Props) {
  const { children: actions, ...actionProps } = Components?.DialogActions || {}

  return (
    <Dialog open={open} onClose={onClose} {...Components?.Dialog}>
      <DialogHeader action={onClose} {...Components?.DialogTitle}>
        {title}
      </DialogHeader>
      <DialogContent {...Components?.DialogContent}>{children}</DialogContent>
      <DialogActions
        sx={(theme) => ({
          mt: 3,
          bgcolor:
            theme.palette.mode === 'dark'
              ? theme.palette.grey[700]
              : theme.palette.grey[200],
        })}
        {...actionProps}
      >
        {actions}
        <Button onClick={onClose} color="error">
          Close
        </Button>
      </DialogActions>
    </Dialog>
  )
}
