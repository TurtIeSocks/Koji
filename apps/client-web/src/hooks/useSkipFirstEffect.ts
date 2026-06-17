import {
  useRef,
  useEffect,
  type DependencyList,
  type EffectCallback,
} from 'react'

export default function useSkipFirstEffect(
  callback: EffectCallback,
  deps: DependencyList,
) {
  const firstRender = useRef(true)

  useEffect(() => {
    if (firstRender.current) {
      firstRender.current = false
      return
    }
    return callback()
  }, deps)
}
