/* eslint-disable react/no-array-index-key */
import React from 'react'
import { Input, List, ListItem, ListItemButton } from '@mui/material'
import Add from '@mui/icons-material/Add'
import Remove from '@mui/icons-material/Remove'

import { usePersist } from '@hooks/usePersist'

import ListSubheader from '../../styled/Subheader'

interface Props<T> {
  value: T
  onChange: (value: T) => void
}

function Color({ value, onChange }: Props<string>) {
  return (
    <Input
      value={value}
      type="color"
      onChange={(e) => onChange(e.target.value)}
      sx={{ width: '33%', mx: 1 }}
    />
  )
}

function Distance({ value, onChange }: Props<number>) {
  return (
    <Input
      value={value}
      type="number"
      onChange={(e) => onChange(+e.target.value)}
      endAdornment="m"
      sx={{ width: '33%', mx: 1 }}
    />
  )
}

export function LineColorSelector() {
  const rules = usePersist((s) => s.lineColorRules)
  const [newColor, setNewColor] = React.useState({
    color: '#000000',
    distance: 0,
  })

  React.useEffect(() => {
    usePersist.setState({
      lineColorRules: rules.sort((a, b) => a.distance - b.distance),
    })
  }, [])

  return (
    <List dense>
      <ListSubheader disableGutters>Distance Colors</ListSubheader>
      <ListItem>
        <Distance
          value={newColor.distance}
          onChange={(e) => setNewColor({ ...newColor, distance: e })}
        />
        <Color
          value={newColor.color}
          onChange={(e) => setNewColor({ ...newColor, color: e })}
        />
        <ListItemButton
          disabled={rules.some((r) => r.distance === newColor.distance)}
          onClick={() => {
            usePersist.setState({
              lineColorRules: [...rules, newColor].sort(
                (a, b) => a.distance - b.distance,
              ),
            })
          }}
        >
          <Add />
        </ListItemButton>
      </ListItem>
      <ListItem sx={{ my: 1 }} />
      {rules.map((rule, i) => (
        <ListItem key={i}>
          <Distance
            value={rule.distance}
            onChange={(e) =>
              usePersist.setState({
                lineColorRules: rules
                  .map((r) =>
                    rule.color === r.color ? { ...r, distance: e } : r,
                  )
                  .sort((a, b) => a.distance - b.distance),
              })
            }
          />
          <Color
            value={rule.color}
            onChange={(e) =>
              usePersist.setState({
                lineColorRules: rules
                  .map((r) =>
                    rule.distance === r.distance ? { ...r, color: e } : r,
                  )
                  .sort((a, b) => a.distance - b.distance),
              })
            }
          />
          <ListItemButton
            onClick={() => {
              usePersist.setState({
                lineColorRules: rules
                  .filter((r) => r.distance !== rule.distance)
                  .sort((a, b) => a.distance - b.distance),
              })
            }}
          >
            <Remove />
          </ListItemButton>
        </ListItem>
      ))}
    </List>
  )
}
