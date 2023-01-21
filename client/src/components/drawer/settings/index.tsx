import * as React from 'react'
import { List, ListItem, ListItemButton, ListSubheader } from '@mui/material'

import Toggle from '../inputs/Toggle'
import MultiOptions from '../inputs/MultiOptions'
import DateTime from '../inputs/DateTime'

export default function Settings() {
  return (
    <List>
      <ListSubheader disableGutters>Markers</ListSubheader>
      <ListItem>
        <MultiOptions field="data" buttons={['all', 'area', 'bound']} />
      </ListItem>
      <DateTime field="last_seen" />
      <Toggle field="pokestop" />
      <Toggle field="gym" />
      <Toggle field="spawnpoint" />
      {process.env.NODE_ENV === 'development' && (
        <Toggle field="nativeLeaflet" />
      )}
      <ListSubheader disableGutters>Other</ListSubheader>
      <Toggle field="loadingScreen" />
      <Toggle field="simplifyPolygons" />
      <ListItemButton href="/config/logout">Logout</ListItemButton>
    </List>
  )
}
