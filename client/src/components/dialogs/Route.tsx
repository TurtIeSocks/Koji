import * as React from 'react'
import TextField from '@mui/material/TextField'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'

import { useImportExport } from '@hooks/useImportExport'

import { ImportExportDialog } from './ImportExport'

function BaseDialog({
  open,
  mode,
}: {
  open: boolean
  mode: 'Import' | 'Export'
}) {
  const stats = useImportExport((s) => s.stats)

  return (
    <ImportExportDialog mode={mode} shape="Route" open={open}>
      <Grid2 xs={3} container justifyContent="flex-start">
        {[
          { label: 'Count', value: stats.count },
          { label: 'Max', value: stats.max?.toFixed(2) },
          {
            label: 'Average',
            value: (stats.total / (stats.count || 1))?.toFixed(2),
          },
          {
            label: 'Total',
            value: stats.total?.toFixed(2),
          },
        ].map((stat, i) => (
          <Grid2 key={stat.label} xs={12}>
            <TextField
              value={stat.value || 0}
              label={stat.label}
              sx={{ width: '90%', my: 2 }}
              disabled
              InputProps={{ endAdornment: i ? 'm' : '' }}
            />
          </Grid2>
        ))}
      </Grid2>
    </ImportExportDialog>
  )
}

export function ImportRoute(): JSX.Element {
  const open = useImportExport((s) => s.open)
  return <BaseDialog open={open === 'importRoute'} mode="Import" />
}

export function ExportRoute() {
  const open = useImportExport((s) => s.open)
  return <BaseDialog open={open === 'exportRoute'} mode="Export" />
}
