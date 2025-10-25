import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export function useCollapsedState() {
  const [isCollapsed, setIsCollapsed] = useState(false)
  const [isLoading, setIsLoading] = useState(true)

  // Função para obter o estado atual
  const getCollapsedState = async () => {
    try {
      const state = await invoke('get_collapsed_state_sync')
      setIsCollapsed(state as boolean)
    } catch (error) {
      console.error('Erro ao obter estado de colapso:', error)
    } finally {
      setIsLoading(false)
    }
  }

  // Função para alternar o estado
  const toggleCollapse = async () => {
    try {
      await invoke('toggle_collapse', { isCollapsed: !isCollapsed })
      setIsCollapsed(!isCollapsed)
    } catch (error) {
      console.error('Erro ao alternar colapso:', error)
    }
  }

  // Carregar estado inicial e atualizar periodicamente
  useEffect(() => {
    getCollapsedState()

    // Atualizar a cada 500ms
    const interval = setInterval(getCollapsedState, 500)
    return () => clearInterval(interval)
  }, [])

  return {
    isCollapsed,
    isLoading,
    toggleCollapse,
    refreshState: getCollapsedState
  }
}
