import * as React from 'react'
import DialogActions from '@mui/material/DialogActions'
import Button from '@mui/material/Button'
import Dialog from '@mui/material/Dialog'
import DialogContent from '@mui/material/DialogContent'
import Grid2 from '@mui/material/Unstable_Grid2/Grid2'
import useDeepCompareEffect from 'use-deep-compare-effect'

import SplitMultiPolygonsBtn from '@components/buttons/SplitMultiPolygons'
import MultiOptions from '@components/drawer/inputs/MultiOptions'
import { CONVERSION_TYPES } from '@assets/constants'
import { useImportExport } from '@hooks/useImportExport'
import ClipboardButton from '@components/drawer/inputs/Clipboard'
import DownloadBtn from '@components/buttons/Download'
import { useShapes } from '@hooks/useShapes'
import { usePersist } from '@hooks/usePersist'

import { Code } from '../Code'
import DialogHeader from './Header'

export function ImportExportDialog({
  open,
  mode,
  shape,
  children,
}: {
  open: boolean
  mode: 'Import' | 'Export'
  shape: 'Polygon' | 'Route'
  children?: React.ReactNode
}) {
  const feature = useImportExport((s) => s.feature)
  const code = useImportExport((s) => s.code)
  const error = useImportExport((s) => s.error)
  const { reset, setCode, fireConvert, updateStats } =
    useImportExport.getState()

  const add = useShapes((s) => s.setters.add)

  const polygonExportMode = usePersist((s) => s.polygonExportMode)
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  React.useEffect(() => {
    if (open && code) {
      fireConvert(mode, shape === 'Route' ? 'MultiPoint' : undefined)
    }
  }, [polygonExportMode, open, code, simplifyPolygons])

  useDeepCompareEffect(() => {
    if (open) {
      if (shape === 'Route') {
        updateStats(mode === 'Export')
      } else {
        setCode(feature)
      }
    }
  }, [open, feature])

  return (
    <Dialog open={open} onClose={reset} maxWidth="xl">
      <DialogHeader action={reset}>
        {mode} {shape}
      </DialogHeader>
      <DialogContent
        sx={{ width: '90vw', minHeight: '60vh', overflow: 'auto' }}
      >
        <Grid2 container>
          <Grid2 xs={children ? 9 : 12} textAlign="left">
            <Code
              code={code}
              setCode={setCode}
              maxHeight="70vh"
              width="100%"
              textMode={
                mode === 'Export'
                  ? polygonExportMode === 'text' ||
                    polygonExportMode === 'altText'
                  : !code.startsWith('{') && !code.startsWith('[')
              }
            />
          </Grid2>
          {children}
        </Grid2>
      </DialogContent>
      <DialogActions>
        <div style={{ flexGrow: 0 }}>
          {mode === 'Export' && (
            <>
              <MultiOptions
                field="polygonExportMode"
                buttons={CONVERSION_TYPES}
                type="select"
              />
              {shape === 'Polygon' && (
                <SplitMultiPolygonsBtn
                  fc={
                    feature.type === 'FeatureCollection'
                      ? feature
                      : { type: 'FeatureCollection', features: [feature] }
                  }
                  setter={setCode}
                />
              )}
            </>
          )}
        </div>
        <div
          style={{ flexGrow: 1, display: 'flex', justifyContent: 'flex-end' }}
        >
          <ClipboardButton text={code} />
          <DownloadBtn data={feature} variant="text" color="primary" />
          <Button
            disabled={!!error}
            onClick={() => {
              if (mode === 'Import' && feature.type === 'FeatureCollection') {
                add(feature.features)
              }
              reset()
            }}
          >
            {mode === 'Import' ? 'Import' : 'Close'}
          </Button>
        </div>
      </DialogActions>
    </Dialog>
  )
}
