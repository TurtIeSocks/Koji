/* eslint-disable no-nested-ternary */
import * as React from 'react'
import {
  ListItem,
  Autocomplete,
  createFilterOptions,
  TextField,
  Checkbox,
  Typography,
  List,
  ListItemText,
  ListItemButton,
  capitalize,
  ListItemIcon,
} from '@mui/material'
import {
  CheckBoxOutlineBlank,
  CheckBox,
  IndeterminateCheckBoxOutlined,
} from '@mui/icons-material'

import { getData } from '@services/fetches'
import { useStatic } from '@hooks/useStatic'
import { useStore } from '@hooks/useStore'
import type { FeatureCollection } from 'geojson'

const icon = <CheckBoxOutlineBlank fontSize="small" color="primary" />
const checkedIcon = <CheckBox fontSize="small" color="primary" />
const partialIcon = (
  <IndeterminateCheckBoxOutlined fontSize="small" color="primary" />
)
const filterOptions = createFilterOptions({
  matchFrom: 'any',
  stringify: (option: string) => option,
})

export default function InstanceSelect() {
  const selected = useStatic((s) => s.selected)
  const instances = useStatic((s) => s.instances)
  const setStatic = useStatic((s) => s.setStatic)
  const setSelected = useStatic((s) => s.setSelected)

  const radius = useStore((s) => s.radius)

  const [inputValue, setInputValue] = React.useState('')
  const [loading, setLoading] = React.useState(false)

  React.useEffect(() => {
    if (!Object.keys(instances).length) {
      setLoading(true)
      getData<FeatureCollection>('/api/instance/all').then((resp) => {
        if (resp) {
          setStatic(
            'instances',
            Object.fromEntries(
              resp.features
                .filter((f) => f.properties?.name)
                .map((f) => [f.properties?.name, f]),
            ),
          )
        }
        setLoading(false)
      })
    }
  }, [])

  return (
    <ListItem>
      <Autocomplete
        value={selected}
        inputValue={inputValue}
        size="small"
        onChange={(_e, newValue) => setSelected(newValue, radius)}
        onInputChange={(_e, newValue) => setInputValue(newValue)}
        filterOptions={filterOptions}
        selectOnFocus
        clearOnBlur
        multiple
        loading={loading}
        handleHomeEndKeys
        disableCloseOnSelect
        groupBy={(option) => instances[option]?.properties?.type}
        sx={{ width: '90%', mx: 'auto' }}
        options={Object.keys(instances).sort((a, b) =>
          instances[a].properties?.type?.localeCompare(
            instances[b].properties?.type,
          ),
        )}
        renderTags={(val) => (
          <Typography align="center">{val.length} Selected</Typography>
        )}
        renderOption={(props, option, { selected: s }) => (
          <li {...props}>
            <Checkbox
              icon={icon}
              checkedIcon={checkedIcon}
              style={{ marginRight: 8 }}
              checked={s}
            />
            {option}
          </li>
        )}
        renderInput={(params) => (
          <TextField label="Select Instance" {...params} />
        )}
        renderGroup={({ key, group, children }) => {
          const allValues = Array.isArray(children)
            ? [...selected, ...children.map((x) => x.key)] // vaguely hacky way to select all filtered results
            : Object.keys(instances).filter((k) => instances[k]?.type === group)
          const allSelected = allValues.every((v) => selected.includes(v))
          const partialSelected =
            allSelected ||
            selected.some((v) => instances[v]?.properties?.type === group)

          return group ? (
            <List key={key}>
              <ListItemButton
                onClick={() => {
                  setSelected(
                    allSelected || partialSelected
                      ? selected.filter(
                          (v) =>
                            !allValues.includes(v) ||
                            instances[v].properties?.type !== group,
                        )
                      : [
                          ...allValues,
                          ...selected.filter(
                            (v) => instances[v].properties?.type !== group,
                          ),
                        ],
                    radius,
                  )
                }}
              >
                <ListItemIcon>
                  {allSelected
                    ? checkedIcon
                    : partialSelected
                    ? partialIcon
                    : icon}
                </ListItemIcon>
                <ListItemText
                  primary={capitalize(
                    group
                      .split('_')
                      .map((x) => capitalize(x))
                      .join(' '),
                  )}
                />
              </ListItemButton>
              {children}
            </List>
          ) : null
        }}
      />
    </ListItem>
  )
}
