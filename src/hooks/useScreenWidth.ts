import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export function useScreenWidth() {
  const [screenWidth, setScreenWidth] = useState(1920)
  const [isLoading, setIsLoading] = useState(true)

  // Função para obter a largura da tela
  const getScreenWidth = async () => {
    try {
      const width = await invoke('get_screen_width')
      setScreenWidth(width as number)
      console.log('🔧 Largura da tela obtida:', width)
    } catch (error) {
      console.error('Erro ao obter largura da tela:', error)
      setScreenWidth(1920) // Fallback para 1920
    } finally {
      setIsLoading(false)
    }
  }

  // Carregar largura inicial
  useEffect(() => {
    getScreenWidth()
  }, [])

  return {
    screenWidth,
    isLoading,
    refreshWidth: getScreenWidth
  }
}
