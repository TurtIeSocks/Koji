import React from 'react'
import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Grid,
  MenuItem,
  Select,
  TextField,
} from '@mui/material'

import { Instance } from '@assets/types'
import { getData } from '@services/utils'
import { useStatic, UseStatic, useStore } from '@hooks/useStore'

interface Props {
  setOpen: UseStatic['setOpen']
}

export default function SelectInstance({ setOpen }: Props) {
  const instanceForm = useStore((s) => s.instanceForm)
  const setInstanceForm = useStore((s) => s.setInstanceForm)
  const open = useStatic((s) => s.open)

  const [instances, setInstances] = React.useState<Instance[]>([])

  const onChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    setInstanceForm({ ...instanceForm, [e.target.name]: +e.target.value })
  }

  React.useEffect(() => {
    getData<Instance[]>('/instances').then((r) =>
      setInstances(r.filter((i) => i.type_ === 'auto_quest')),
    )
  }, [])

  return (
    <Dialog open={open === 'instance'}>
      <DialogTitle>Select Instance</DialogTitle>
      <DialogContent>
        <Grid
          container
          rowSpacing={2}
          alignItems="center"
          justifyContent="space-evenly"
        >
          <Grid item xs={11}>
            <Select
              fullWidth
              value={instanceForm.name}
              onChange={(e) => {
                setInstanceForm({ ...instanceForm, name: e.target.value })
              }}
            >
              {instances.map((instance) => (
                <MenuItem key={instance.name} value={instance.name}>
                  {instance.name}
                </MenuItem>
              ))}
            </Select>
          </Grid>
          <Grid item xs={5}>
            <TextField
              name="radius"
              fullWidth
              value={instanceForm.radius}
              type="number"
              label="Radius"
              onChange={onChange}
            />
          </Grid>
          <Grid item xs={5}>
            <TextField
              name="generations"
              fullWidth
              value={instanceForm.generations}
              type="number"
              label="Generations"
              onChange={onChange}
            />
          </Grid>
        </Grid>
      </DialogContent>
      <DialogActions>
        <Button onClick={() => setOpen('')}>Close</Button>
      </DialogActions>
    </Dialog>
  )
}
