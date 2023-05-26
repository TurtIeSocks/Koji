import * as React from 'react'
import {
  Collapse,
  Divider,
  List,
  ListItem,
  ListItemText,
  MenuItem,
  Select,
} from '@mui/material'

import { BOOTSTRAP_LEVELS, S2_CELL_LEVELS } from '@assets/constants'
import { usePersist } from '@hooks/usePersist'

import { MultiOptionList } from './inputs/MultiOptions'
import DateTime from './inputs/DateTime'
import Toggle from './inputs/Toggle'
import ListSubheader from '../styled/Subheader'

export default function Layers() {
  const s2cells = usePersist((s) => s.s2cells)
  const calculationMode = usePersist((s) => s.calculation_mode)
  const s2DisplayMode = usePersist((s) => s.s2DisplayMode)
  const spawnpoint = usePersist((s) => s.spawnpoint)

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
      <Collapse in={spawnpoint}>
        <MultiOptionList
          field="tth"
          buttons={['All', 'Known', 'Unknown']}
          type="select"
        />
      </Collapse>
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
      <ListItemText
        inset
        secondary="Local timezone, sent in UTC to server"
        secondaryTypographyProps={{ variant: 'caption' }}
        sx={{ my: 0 }}
      />
      <Divider sx={{ my: 2 }} />
      <ListSubheader disableGutters>S2 Cells</ListSubheader>
      <MultiOptionList
        field="s2DisplayMode"
        buttons={['none', 'covered', 'all']}
        label="Display"
        hideLabel
        type="select"
      />
      <Collapse in={s2DisplayMode !== 'none'}>
        <MultiOptionList
          field="calculation_mode"
          buttons={['Radius', 'S2']}
          label="Mode"
          hideLabel
          type="select"
        />
        <MultiOptionList
          field="s2FillMode"
          buttons={['simple', 'all']}
          label="Fill"
          hideLabel
          type="select"
        />
      </Collapse>
      <Collapse in={calculationMode === 'Radius'}>
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
      <Collapse in={calculationMode === 'S2'}>
        <MultiOptionList
          field="s2_level"
          label="S2 Level"
          hideLabel
          buttons={S2_CELL_LEVELS}
          type="select"
          itemLabel={(v) => `Level ${v}`}
        />
        <MultiOptionList
          field="s2_size"
          label="S2 Size"
          hideLabel
          buttons={BOOTSTRAP_LEVELS}
          type="select"
          itemLabel={(v) => `${v}x${v}`}
        />
      </Collapse>
    </List>
  )
}
