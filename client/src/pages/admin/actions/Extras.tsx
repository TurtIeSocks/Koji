import * as React from 'react'
import { DeleteWithUndoButton } from 'react-admin'
import { IconButton, Menu, MenuItem } from '@mui/material'
import MoreVertIcon from '@mui/icons-material/MoreVert'

import { ExportButton } from './Export'
import { PushToProd } from './PushToApi'

export function ExtraMenuActions({ resource }: { resource: string }) {
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null)

  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation()
    setAnchorEl(null)
  }

  return (
    <>
      <IconButton
        onClick={(e) => {
          e.stopPropagation()
          setAnchorEl(e.currentTarget)
        }}
      >
        <MoreVertIcon />
      </IconButton>
      <Menu open={!!anchorEl} anchorEl={anchorEl} onClose={handleClose}>
        <MenuItem onClick={handleClose}>
          <DeleteWithUndoButton />
        </MenuItem>
        <MenuItem onClick={handleClose}>
          <ExportButton resource={resource} />
        </MenuItem>
        <MenuItem
          onClick={handleClose}
          sx={{ display: { xs: 'flex', sm: 'none' } }}
        >
          <PushToProd resource={resource} />
        </MenuItem>
      </Menu>
    </>
  )
}
