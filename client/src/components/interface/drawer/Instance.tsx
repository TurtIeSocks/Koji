import * as React from 'react'
import {
  ListItem,
  Autocomplete,
  createFilterOptions,
  TextField,
  CircularProgress,
} from '@mui/material'

import type { UseStore } from '@hooks/useStore'
import { getData } from '@services/fetches'

interface Props {
  value: string
  setValue: (field: keyof UseStore, value: string) => void
}

export default function InstanceSelect({ value, setValue }: Props) {
  const [instances, setInstances] = React.useState<string[]>([])

  React.useEffect(() => {
    getData<string[]>('/api/instance/type/auto_quest').then((r) =>
      setInstances(r || []),
    )
  }, [])

  return (
    <ListItem>
      {instances.length ? (
        <Autocomplete
          value={instances.includes(value) ? value : ''}
          size="small"
          onChange={(_e, newValue) => {
            if (newValue) setValue('instance', newValue)
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
          handleHomeEndKeys
          sx={{ width: '90%', mx: 'auto' }}
          options={['', ...instances]}
          renderOption={(props, option) => <li {...props}>{option}</li>}
          renderInput={(params) => (
            <TextField label="Select Instance" {...params} />
          )}
        />
      ) : (
        <CircularProgress color="secondary" size={100} />
      )}
    </ListItem>
  )
}
