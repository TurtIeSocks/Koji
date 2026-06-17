import { useEffect, useState } from 'react'
import { useStatic } from './useStatic'

export function useAlertTimer() {
  const { status } = useStatic((s) => s.notification)
  const [hover, setHover] = useState(false)

  useEffect(() => {
    if (status && !hover) {
      const timer = setTimeout(() => {
        useStatic.setState((prev) => ({
          notification: { ...prev.notification, message: '', status: 0 },
        }))
      }, 5000)
      return () => clearTimeout(timer)
    }
  }, [status, hover])

  return setHover
}
