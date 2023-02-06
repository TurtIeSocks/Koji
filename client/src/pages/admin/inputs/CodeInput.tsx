import { GEOMETRY_CONVERSION_TYPES } from '@assets/constants'
import { ConversionOptions, Conversions } from '@assets/types'
import { Code } from '@components/Code'
import { usePersist } from '@hooks/usePersist'
import { Typography } from '@mui/material'
import { convert } from '@services/fetches'
import { safeParse } from '@services/utils'
import * as React from 'react'
import { useInput } from 'react-admin'

export default function CodeInput({
  source,
  label,
  conversionType,
  geometryType,
}: {
  source: string
  label?: string
  conversionType?: ConversionOptions
  geometryType?: typeof GEOMETRY_CONVERSION_TYPES[number]
}) {
  const { field } = useInput({ source })
  const [error, setError] = React.useState('')
  const simplifyPolygons = usePersist((s) => s.simplifyPolygons)

  return (
    <>
      <Typography variant="subtitle2">{label}</Typography>
      <Code
        width="75vw"
        maxHeight="50vh"
        code={
          typeof field.value === 'string' && field.value
            ? JSON.stringify(JSON.parse(field.value), null, 2)
            : JSON.stringify(field.value, null, 2)
        }
        setCode={(newCode) => {
          field.onChange({ target: { value: newCode } })
        }}
        onBlurCapture={async () => {
          if (conversionType) {
            const geofence = safeParse<Conversions>(field.value)
            const parsed = geofence.error ? field.value : geofence.value

            const type =
              parsed?.geometry?.type ||
              (parsed?.type !== 'Feature'
                ? parsed?.type || geometryType
                : geometryType)
            await convert(parsed, conversionType, simplifyPolygons, type).then(
              (res) => {
                if (Array.isArray(res)) {
                  setError(
                    'Warning, multiple features were found, you should only assign one feature!',
                  )
                } else {
                  field.onChange({
                    target: { value: JSON.stringify(res, null, 2) },
                  })
                  setError('')
                }
              },
            )
          }
        }}
      />
      <Typography color="error">{error}</Typography>
    </>
  )
}
