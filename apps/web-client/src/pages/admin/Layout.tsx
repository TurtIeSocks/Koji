import * as React from 'react'
import { Layout as BaseLayout, type LayoutProps } from 'react-admin'

import AppBar from './AppBar'

export default function Layout(props: LayoutProps) {
  return <BaseLayout {...props} appBar={AppBar} />
}
