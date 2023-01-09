import * as React from 'react'
import { FixedSizeList, type ListChildComponentProps } from 'react-window'

export default function ReactWindow<T, U>({
  children,
  rows,
  data,
  itemSize,
  height = 500,
  width = '100%',
}: {
  children: React.FC<ListChildComponentProps<U & { rows: T[] }>>
  rows: T[]
  data: U
  itemSize?: number
  height?: number
  width?: number | string
}) {
  return (
    <FixedSizeList
      height={height}
      width={width}
      itemCount={rows.length}
      itemSize={itemSize || 40}
      itemData={{ rows, ...data }}
    >
      {children}
    </FixedSizeList>
  )
}
