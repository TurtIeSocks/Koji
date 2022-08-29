import * as React from 'react'
import { Dialog, DialogContent, DialogActions, Button } from '@mui/material'

import type { Feature } from 'geojson'

import { useStore } from '@hooks/useStore'
import { geojsonToExport } from '@services/utils'

import DialogHeader from './Header'
import { Code } from '../Code'
import BtnGroup from '../drawer/inputs/BtnGroup'

interface Props {
  open: boolean
  setOpen: (open: boolean) => void
  feature: Feature
}

export default function ExportPolygon({ open, setOpen, feature }: Props) {
  const polygonExportMode = useStore((s) => s.polygonExportMode)
  const setStore = useStore((s) => s.setStore)
  const [code, setCode] = React.useState(JSON.stringify(feature, null, 2))

  React.useEffect(() => {
    const array = geojsonToExport(feature)
    const object = geojsonToExport(feature, true)
    switch (polygonExportMode) {
      case 'geojson':
        setCode(JSON.stringify(feature, null, 2))
        break
      case 'array':
        setCode(
          JSON.stringify(
            feature.geometry.type === 'Polygon' ? array[0] : array,
            null,
            2,
          ),
        )
        break
      case 'object':
        setCode(
          JSON.stringify(
            feature.geometry.type === 'Polygon' ? object[0] : object,
            null,
            2,
          ),
        )
        break
      default:
        break
    }
  }, [polygonExportMode])
  return (
    <Dialog open={open} onClose={() => setOpen(false)} maxWidth="xl">
      <DialogHeader title="Export Polygon" action={() => setOpen(false)} />
      <DialogContent sx={{ width: '90vw', height: '60vh', overflow: 'auto' }}>
        <Code code={code} setCode={setCode} />
      </DialogContent>
      <DialogActions>
        <BtnGroup
          field="polygonExportMode"
          value={polygonExportMode}
          setValue={setStore}
          buttons={['array', 'geojson', 'object']}
        />
        <Button onClick={() => navigator.clipboard.writeText(code)}>
          Copy to Clipboard
        </Button>
        <Button onClick={() => setOpen(false)}>Close</Button>
      </DialogActions>
    </Dialog>
  )
}
