import * as React from 'react'
import {
  Divider,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  MenuItem,
  Select,
} from '@mui/material'
import Logout from '@mui/icons-material/Logout'
import Keyboard from '@mui/icons-material/Keyboard'
import InfoIcon from '@mui/icons-material/Info'
import FavoriteIcon from '@mui/icons-material/Favorite'
import { useNavigate } from 'react-router'

import { KojiResponse, KojiTileServer } from '@assets/types'
import { useStatic } from '@hooks/useStatic'
import { usePersist } from '@hooks/usePersist'
import { fetchWrapper } from '@services/fetches'

import Toggle from './inputs/Toggle'
import ListSubheader from '../styled/Subheader'
import NumInput from './inputs/NumInput'

export default function Settings() {
  const navigate = useNavigate()

  const tileServers = useStatic((s) => s.tileServers)
  const tileServer = usePersist((s) => s.tileServer)

  React.useEffect(() => {
    fetchWrapper<KojiResponse<KojiTileServer[]>>(
      '/internal/admin/tileserver/all/',
    ).then((data) => data && useStatic.setState({ tileServers: data.data }))
  }, [])

  return (
    <List>
      <ListSubheader disableGutters>Settings</ListSubheader>
      <Toggle field="loadingScreen" />
      <Toggle field="simplifyPolygons" />
      <Toggle field="showRouteIndex" />
      <Toggle field="scaleMarkers" />
      <ListItemButton
        onClick={() =>
          useStatic.setState((prev) => ({
            dialogs: {
              ...prev.dialogs,
              keyboard: true,
            },
          }))
        }
      >
        <ListItemText primary="Keyboard Shortcuts" />
        <ListItemIcon>
          <Keyboard />
        </ListItemIcon>
      </ListItemButton>
      {tileServers.length ? (
        <ListItem>
          <Select
            value={tileServer}
            fullWidth
            onChange={({ target }) => {
              usePersist.setState({ tileServer: target.value })
            }}
          >
            {tileServers.map(({ id, name, url }) => (
              <MenuItem key={id} value={url}>
                {name}
              </MenuItem>
            ))}
          </Select>
        </ListItem>
      ) : (
        <ListItemText
          primary="Set TileServer"
          secondary="Upload from the admin panel"
          sx={{ px: 2 }}
        />
      )}
      {process.env.NODE_ENV === 'development' && (
        <>
          <Divider sx={{ my: 2 }} />
          <ListSubheader disableGutters>Dev</ListSubheader>
          <Toggle field="nativeLeaflet" />
          <Toggle field="colorByGeohash" />
          <NumInput field="geohashPrecision" />
        </>
      )}
      <Divider sx={{ my: 2 }} />
      <ListItemButton
        href="https://koji.vercel.app/"
        target="_blank"
        referrerPolicy="no-referrer"
      >
        <ListItemIcon>
          <InfoIcon color="primary" />
        </ListItemIcon>
        <ListItemText
          primary="Documentation"
          primaryTypographyProps={{ color: 'primary' }}
        />
      </ListItemButton>
      <ListItemButton
        href="https://github.com/sponsors/TurtIeSocks"
        target="_blank"
        referrerPolicy="no-referrer"
      >
        <ListItemIcon>
          <FavoriteIcon color="primary" />
        </ListItemIcon>
        <ListItemText
          primary="Save a Turtle"
          primaryTypographyProps={{ color: 'primary' }}
        />
      </ListItemButton>
      <ListItemButton onClick={() => navigate('/admin')}>
        <ListItemIcon>
          <Logout color="secondary" />
        </ListItemIcon>
        <ListItemText
          primary="Admin Panel"
          primaryTypographyProps={{ color: 'secondary' }}
        />
      </ListItemButton>
      <ListItemButton href="/config/logout">
        <ListItemIcon>
          <Logout color="secondary" />
        </ListItemIcon>
        <ListItemText
          primary="Logout"
          primaryTypographyProps={{ color: 'secondary' }}
        />
      </ListItemButton>
    </List>
  )
}
