/* eslint-disable react/no-array-index-key */
import * as React from 'react'
import {
  Dialog,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  List,
  ListItemText,
  ListSubheader,
} from '@mui/material'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

import { useStore } from '@hooks/useStore'
import DialogHeader from './Header'

interface Props {
  open: string
  setOpen: (open: string) => void
}

export default function ExportRoute({ open, setOpen }: Props) {
  const exportSettings = useStore((s) => s.export)

  const total = exportSettings.route.flatMap((route) => route).length
  return (
    <Dialog open={open === 'export'} onClose={() => setOpen('')}>
      <DialogHeader title="Export Route" action={() => setOpen('')} />
      <DialogContent>
        <Grid2 container>
          <Grid2
            container
            xs={7}
            height="50vh"
            overflow="auto"
            border="2px grey solid"
            borderRadius={2}
            mx={2}
            alignItems="center"
            justifyContent="center"
          >
            <List>
              {exportSettings.route.map((route, i) => (
                <React.Fragment key={i}>
                  <ListSubheader>Device {i + 1}</ListSubheader>
                  {route.map((point, j) => (
                    <ListItemText
                      key={`${i}-${j}-${point.join('')}`}
                      primary={`${point[0]}, ${point[1]}`}
                      primaryTypographyProps={{ variant: 'caption' }}
                    />
                  ))}
                </React.Fragment>
              ))}
            </List>
          </Grid2>
          <Grid2
            container
            xs={4}
            direction="column"
            alignItems="center"
            justifyContent="space-around"
            height="50vh"
          >
            <Grid2>
              <TextField
                value={total || 0}
                label="Count"
                type="number"
                fullWidth
                disabled
              />
            </Grid2>
            <Grid2>
              <TextField
                value={exportSettings.max?.toFixed(2) || 0}
                label="Max"
                type="number"
                fullWidth
                InputProps={{ endAdornment: 'm' }}
                disabled
              />
            </Grid2>
            <Grid2>
              <TextField
                value={(exportSettings.total / total)?.toFixed(2) || 0}
                label="Average"
                type="number"
                fullWidth
                InputProps={{ endAdornment: 'm' }}
                disabled
              />
            </Grid2>
            <Grid2>
              <TextField
                value={exportSettings.total?.toFixed(2) || 0}
                label="Total"
                type="number"
                fullWidth
                InputProps={{ endAdornment: 'm' }}
                disabled
              />
            </Grid2>
          </Grid2>
        </Grid2>
      </DialogContent>
      <DialogActions>
        <Button
          onClick={() =>
            navigator.clipboard.writeText(
              exportSettings.route
                .map((r) => r.map((p) => p.join(',')).join('\n'))
                .join('\n\n'),
            )
          }
        >
          Copy to Clipboard
        </Button>
        <Button onClick={() => setOpen('')}>Close</Button>
      </DialogActions>
    </Dialog>
  )
}
