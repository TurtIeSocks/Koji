import * as React from 'react'

import { useImportExport } from '@hooks/useImportExport'

import { ImportExportDialog } from './ImportExport'

export function ImportPolygon() {
  const open = useImportExport((s) => s.open)
  return (
    <ImportExportDialog
      open={open === 'importPolygon'}
      mode="Import"
      shape="Polygon"
    />
  )
}

export function ExportPolygon(): JSX.Element {
  const open = useImportExport((s) => s.open)
  return (
    <ImportExportDialog
      open={open === 'exportPolygon'}
      mode="Export"
      shape="Polygon"
    />
  )
}
