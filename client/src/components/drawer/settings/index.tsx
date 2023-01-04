import * as React from 'react'
import { List, ListItem, ListItemButton, ListSubheader } from '@mui/material'

import Toggle from '../inputs/Toggle'
import MultiOptions from '../inputs/MultiOptions'
import DateTime from '../inputs/DateTime'

export default function Settings() {
  return (
    <List>
      <ListSubheader disableGutters>Markers</ListSubheader>
      <DateTime field="last_seen" />
      <Toggle field="pokestop" />
      <Toggle field="gym" />
      <Toggle field="spawnpoint" />
      <Toggle field="nativeLeaflet" />
      <Toggle field="loadingScreen" />
      <Toggle field="simplifyPolygons" />
      <ListItem>
        <MultiOptions field="data" buttons={['all', 'area', 'bound']} />
      </ListItem>
      <ListItemButton href="/config/logout">Logout</ListItemButton>
    </List>
  )
}
