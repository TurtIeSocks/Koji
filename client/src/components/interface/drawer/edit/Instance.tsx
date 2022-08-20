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
import type { UseStore } from '@hooks/useStore'
import { getData } from '@services/fetches'
import { useStatic } from '@hooks/useStatic'
import { type Instance } from '@assets/types'

interface Props {
  value: string[]
  setValue: (field: keyof UseStore, value: string[]) => void
}

const icon = <CheckBoxOutlineBlank fontSize="small" color="primary" />
const checkedIcon = <CheckBox fontSize="small" color="primary" />
const partialIcon = (
  <IndeterminateCheckBoxOutlined fontSize="small" color="primary" />
)

export default function InstanceSelect({ value, setValue }: Props) {
  const instances = useStatic((s) => s.instances)
  const validInstances = useStatic((s) => s.validInstances)
  const scannerType = useStatic((s) => s.scannerType)
  const setStatic = useStatic((s) => s.setStatic)

  const [loading, setLoading] = React.useState(false)
  React.useEffect(() => {
    if (!validInstances.size && scannerType === 'rdm') {
      setLoading(true)
      getData<Instance[]>('/api/instance/all').then((r) => {
        setStatic(
          'instances',
          Array.isArray(r) ? Object.fromEntries(r.map((x) => [x.name, x])) : {},
        )
        setStatic(
          'validInstances',
          Array.isArray(r) ? new Set(r.map((v) => v.name)) : new Set(),
        )
        setLoading(false)
      })
    }
  }, [])

  React.useEffect(() => {
    if (
      !Array.isArray(value) ||
      (validInstances.size && value.some((v) => !validInstances.has(v)))
    ) {
      setValue('instance', [])
    }
  }, [value, validInstances])

  return (
    <ListItem>
      <Autocomplete
        value={
          Array.isArray(value) ? value.filter((x) => validInstances.has(x)) : []
        }
        size="small"
        onChange={(_e, newValue) => {
          setValue('instance', newValue)
        }}
        filterOptions={(opts, params) => {
          const filtered = createFilterOptions<string>()(opts, params)
          const { inputValue } = params
          const isExisting = opts.some((option) => inputValue === option)
          if (inputValue !== '' && !isExisting) {
            filtered.push(inputValue)
          }
          return filtered
        }}
        selectOnFocus
        clearOnBlur
        multiple
        loading={loading}
        handleHomeEndKeys
        disableCloseOnSelect
        groupBy={(option) => instances[option]?.type_}
        sx={{ width: '90%', mx: 'auto' }}
        options={[...validInstances].sort((a, b) =>
          instances[a].type_.localeCompare(instances[b].type_),
        )}
        renderTags={(val) => (
          <Typography align="center">{val.length} Selected</Typography>
        )}
        renderOption={(props, option, { selected }) => (
          <li {...props}>
            <Checkbox
              icon={icon}
              checkedIcon={checkedIcon}
              style={{ marginRight: 8 }}
              checked={selected}
            />
            {option}
          </li>
        )}
        renderInput={(params) => (
          <TextField label="Select Instance" {...params} />
        )}
        renderGroup={({ key, group, children }) => {
          const allValues = Object.keys(instances).filter(
            (k) => instances[k]?.type_ === group,
          )
          const allSelected = allValues.every((v) => value.includes(v))
          const partialSelected =
            allSelected || value.some((v) => allValues.includes(v))

          return (
            <List key={key}>
              <ListItemButton
                onClick={() => {
                  if (allSelected || partialSelected) {
                    setValue(
                      'instance',
                      value.filter(
                        (v) =>
                          !allValues.includes(v) ||
                          instances[v].type_ !== group,
                      ),
                    )
                  } else {
                    setValue('instance', [
                      ...allValues,
                      ...value.filter((v) => instances[v].type_ !== group),
                    ])
                  }
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
          )
        }}
      />
    </ListItem>
  )
}
