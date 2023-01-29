import Button, { ButtonProps } from '@mui/material/Button/Button'
import { combineByProperty } from '@services/utils'
import type { FeatureCollection } from '@assets/types'
import * as React from 'react'

interface Props extends ButtonProps {
  fc: FeatureCollection
  nameProp: string
  setter: (fc: FeatureCollection) => void
  disabled: boolean
}

export default function CombineByName({
  fc,
  nameProp,
  setter,
  ...rest
}: Props) {
  return (
    <Button
      color="error"
      onClick={() => {
        const newFc = combineByProperty(fc, nameProp)
        setter(newFc)
      }}
      {...rest}
    >
      Combine by Name Key
    </Button>
  )
}
