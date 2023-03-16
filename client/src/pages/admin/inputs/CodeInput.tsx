import * as React from 'react'
import { useInput } from 'react-admin'
import useDeepCompareEffect from 'use-deep-compare-effect'
import { Typography, CircularProgress } from '@mui/material'

import { GEOMETRY_CONVERSION_TYPES } from '@assets/constants'
import { ConversionOptions, Conversions } from '@assets/types'
import { Code } from '@components/Code'
import { usePersist } from '@hooks/usePersist'
import { convert } from '@services/fetches'
import { safeParse } from '@services/utils'

function getSafeString(value: string | object) {
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value, null, 2)
  } catch (e) {
    return ''
  }
}

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
  const [tempValue, setTempValue] = React.useState('')
  const [loading, setLoading] = React.useState(false)

  const { simplifyPolygons } = usePersist.getState()

  useDeepCompareEffect(() => {
    setTempValue(getSafeString(field.value))
  }, [{ value: field.value }])

  return (
    <>
      <Typography variant="subtitle2">{label}</Typography>
      {loading ? (
        <CircularProgress />
      ) : (
        <Code
          width="75vw"
          maxHeight="50vh"
          code={tempValue}
          setCode={(newCode) => setTempValue(newCode)}
          onBlurCapture={async () => {
            if (conversionType) {
              const geofence = safeParse<Conversions>(tempValue)
              const parsed = geofence.error ? tempValue : geofence.value

              if (parsed) {
                const type =
                  typeof parsed === 'object' &&
                  !Array.isArray(parsed) &&
                  parsed.type
                    ? parsed.type
                    : geometryType
                setLoading(true)
                await convert(
                  parsed,
                  conversionType,
                  simplifyPolygons,
                  type as typeof geometryType,
                ).then((res) => {
                  if (Array.isArray(res)) {
                    setError(
                      'Warning, multiple features were found, you should only assign one feature!',
                    )
                  } else {
                    field.onChange({
                      target: { value: res },
                    })
                    setError('')
                  }
                  setLoading(false)
                })
              }
            }
          }}
        />
      )}

      <Typography color="error">{error}</Typography>
    </>
  )
}
