import * as React from 'react'

export default function TabPanel({
  children,
  value,
  index,
  ...other
}: {
  children?: React.ReactNode
  dir?: string
  index: number
  value: number
}) {
  return (
    <div role="tabpanel" hidden={value !== index} {...other}>
      {value === index && children}
    </div>
  )
}
