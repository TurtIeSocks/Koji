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

import { getData } from '@services/fetches'
import { useStatic, useStore } from '@hooks/useStore'

export default function SelectInstance() {
  const instanceForm = useStore((s) => s.instanceForm)
  const setInstanceForm = useStore((s) => s.setInstanceForm)
  const open = useStatic((s) => s.open)
  const handleClose = useStatic((s) => s.handleClose)

  const [instances, setInstances] = React.useState<string[]>([])

  const onChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => {
    setInstanceForm({ ...instanceForm, [e.target.name]: +e.target.value || '' })
  }

  React.useEffect(() => {
    getData<string[]>('/api/instance/quest').then((r) => setInstances(r || []))
  }, [])

  return (
    <Dialog open={open === 'instance'} onClose={() => handleClose()}>
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
                <MenuItem key={instance} value={instance}>
                  {instance}
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
        <Button onClick={() => handleClose()}>Close</Button>
      </DialogActions>
    </Dialog>
  )
}
