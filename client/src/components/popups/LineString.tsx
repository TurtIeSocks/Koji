import * as React from 'react'
import type { PopupProps } from '@assets/types'

interface Props extends PopupProps {
  dis: number
}

export default function LineStringPopup({ id, properties, dis }: Props) {
  return (
    <>
      {JSON.stringify({ id, properties }, null, 2)}
      {dis.toFixed(2)}
    </>
  )
}
