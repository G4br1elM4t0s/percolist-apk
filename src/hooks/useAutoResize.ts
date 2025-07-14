import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'

export function useAutoResize(dependencies: any[] = []) {
  const contentRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const adjustHeight = async () => {
      if (contentRef.current) {
        const height = contentRef.current.scrollHeight
        try {
          await invoke('adjust_window_height', { height })
        } catch (error) {
          console.error('Failed to adjust window height:', error)
        }
      }
    }

    // Ajusta a altura inicial
    adjustHeight()

    // Cria um observer para monitorar mudanças no conteúdo
    const resizeObserver = new ResizeObserver(() => {
      adjustHeight()
    })

    if (contentRef.current) {
      resizeObserver.observe(contentRef.current)
    }

    return () => {
      resizeObserver.disconnect()
    }
  }, dependencies)

  return contentRef
}
