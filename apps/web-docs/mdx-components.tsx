import { useMDXComponents as getThemeMDXComponents } from 'nextra-theme-docs'

export function useMDXComponents(components = {}) {
  return getThemeMDXComponents(components)
}
