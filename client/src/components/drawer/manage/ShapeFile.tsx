/* eslint-disable no-console */
import { Button, CircularProgress } from '@mui/material'
import * as React from 'react'
import * as shapefile from 'shapefile'
import UploadFileIcon from '@mui/icons-material/UploadFile'
import type { Feature, FeatureCollection } from 'geojson'
import { usePersist } from '@hooks/usePersist'
import { convert } from '@services/fetches'

interface Props {
  setter?: (featureCollection: FeatureCollection) => void
}

export default function ShapeFile({ setter }: Props) {
  const [shpString, setShpString] = React.useState<string | ArrayBuffer>('')
  const [dbfString, setDbfString] = React.useState<string | ArrayBuffer>('')
  const [fileNames, setFileNames] = React.useState<string[]>([])
  const [loading, setLoading] = React.useState<boolean>(false)

  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!e.target.files) {
      return
    }

    Array.from(e.target.files).forEach((file) => {
      setFileNames((prev) => [...prev, file.name])
      const reader = new FileReader()
      reader.onload = (evt) => {
        if (!evt?.target?.result) {
          return
        }
        if (file.name.endsWith('.shp')) {
          setShpString(evt.target.result)
        }
        if (file.name.endsWith('.dbf')) {
          setDbfString(evt.target.result)
        }
      }
      reader.readAsArrayBuffer(file)
    })
  }

  React.useEffect(() => {
    const values: Feature[] = []
    if (shpString) {
      shapefile
        .open(shpString, dbfString || undefined)
        .then((source) =>
          source.read().then(function write(result): Promise<void> | void {
            if (result.done && setter) {
              setLoading(true)
              console.log('ShapeFile Results:', values)
              return convert<FeatureCollection>(
                values,
                'featureCollection',
                simplifyPolygons,
              ).then((geo) => {
                setLoading(false)
                setter(geo)
              })
            }
            values.push(result.value)
            return source.read().then(write)
          }),
        )
        .catch((error) => console.error(error.stack))
    }
  }, [shpString, dbfString])

  return loading ? (
    <CircularProgress color="secondary" />
  ) : (
    <Button
      component="label"
      variant="contained"
      color="secondary"
      sx={{ maxWidth: '90%' }}
      endIcon={fileNames.length ? undefined : <UploadFileIcon />}
      onClick={() => setFileNames([])}
    >
      {fileNames.length > 0
        ? fileNames
            .map((name) =>
              name.length > 15 ? `${name.substring(0, 15)}...` : name,
            )
            .join(', ')
        : 'Browse'}
      <input
        type="file"
        hidden
        multiple
        accept=".shp,.dbf"
        onChange={handleFileUpload}
      />
    </Button>
  )
}
