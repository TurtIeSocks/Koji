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
import { type Instance } from '@assets/types'
import { useStore } from '@hooks/useStore'

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
  const scannerType = useStatic((s) => s.scannerType)
  const setStatic = useStatic((s) => s.setStatic)
  const setSelected = useStatic((s) => s.setSelected)

  const radius = useStore((s) => s.radius)

  const [inputValue, setInputValue] = React.useState('')
  const [loading, setLoading] = React.useState(false)

  React.useEffect(() => {
    if (!Object.keys(instances).length && scannerType === 'rdm') {
      setLoading(true)
      getData<Instance[]>('/api/instance/all').then((r) => {
        setStatic(
          'instances',
          Array.isArray(r) ? Object.fromEntries(r.map((x) => [x.name, x])) : {},
        )
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
        groupBy={(option) => instances[option]?.type}
        sx={{ width: '90%', mx: 'auto' }}
        options={Object.keys(instances).sort((a, b) =>
          instances[a].type?.localeCompare(instances[b].type),
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
            allSelected || selected.some((v) => allValues.includes(v))

          return group ? (
            <List key={key}>
              <ListItemButton
                onClick={() => {
                  setSelected(
                    allSelected || partialSelected
                      ? selected.filter(
                          (v) =>
                            !allValues.includes(v) ||
                            instances[v].type !== group,
                        )
                      : [
                          ...allValues,
                          ...selected.filter(
                            (v) => instances[v].type !== group,
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
