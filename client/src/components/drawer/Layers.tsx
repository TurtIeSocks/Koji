import * as React from 'react'
import {
  Collapse,
  Divider,
  List,
  ListItem,
  MenuItem,
  Select,
} from '@mui/material'

import { S2_CELL_LEVELS } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'

import { MultiOptionList } from './inputs/MultiOptions'
import DateTime from './inputs/DateTime'
import Toggle from './inputs/Toggle'
import ListSubheader from '../styled/Subheader'

export default function Layers() {
  const s2cells = usePersist((s) => s.s2cells)
  const bootstrap_mode = usePersist((s) => s.bootstrap_mode)

  return (
    <List dense>
      <ListSubheader disableGutters>Vectors</ListSubheader>
      <Toggle field="showCircles" />
      <Toggle field="showLines" />
      <Toggle field="showPolygons" />
      <Toggle field="showArrows" />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>Markers</ListSubheader>
      <Toggle field="gym" />
      <Toggle field="spawnpoint" />
      <Toggle field="pokestop" />
      <Toggle field="pokestopRange" />
      <MultiOptionList
        field="data"
        buttons={['all', 'area', 'bound']}
        label="Query Type"
        hideLabel
        type="select"
      />
      <DateTime field="last_seen" />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>S2 Cells</ListSubheader>
      <Toggle field="fillCoveredCells" />
      <MultiOptionList
        field="bootstrap_mode"
        buttons={['Radius', 'S2']}
        label="Mode"
        hideLabel
        type="select"
      />
      <Collapse in={bootstrap_mode === 'Radius'}>
        <ListItem>
          <Select
            fullWidth
            value={s2cells}
            multiple
            onChange={({ target }) =>
              usePersist.setState({
                s2cells:
                  typeof target.value === 'string'
                    ? target.value.split(',').map((val) => +val)
                    : target.value,
              })
            }
          >
            {S2_CELL_LEVELS.map((level) => (
              <MenuItem key={level} value={level}>
                Level {level}
              </MenuItem>
            ))}
          </Select>
        </ListItem>
      </Collapse>
    </List>
  )
}
