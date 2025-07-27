import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useUserStore } from '../store'

interface AuthData {
  token: string
  sessionId: string
  authenticated: boolean
}

export const useAuthBlock = () => {
  const { user, setUser, setAuth, setAuthenticated, clearUser, isBlocked } = useUserStore()
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Função para carregar dados do usuário do localStorage na inicialização
  const loadUserDataFromStorage = () => {
    try {
      const authData = localStorage.getItem('percolist_auth')
      const userData = localStorage.getItem('percolist_user')

      if (authData) {
        const parsedAuth = JSON.parse(authData)
        setAuth(parsedAuth)
        setAuthenticated(true)
        console.log('✅ Auth data carregado do localStorage:', parsedAuth)
      }

      if (userData) {
        const parsedUser = JSON.parse(userData)
        setUser(parsedUser)
        console.log('✅ User data carregado do localStorage:', parsedUser)
      }
    } catch (err) {
      console.error('❌ Erro ao carregar dados do localStorage:', err)
      setError('Erro ao carregar dados salvos')
    }
  }

  // Função para sincronizar dados com a API
  const syncUserData = async () => {
    try {
      const authData = localStorage.getItem('percolist_auth')
      if (!authData) {
        console.log('ℹ️ Nenhum auth data encontrado para sincronizar')
        return false
      }

      const parsedAuth = JSON.parse(authData)
      console.log('🔄 Sincronizando dados com API...', parsedAuth)

      // Simular chamada para API
      const response = await fetch('http://localhost:3000/auth/sync', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${parsedAuth.token}`
        },
        body: JSON.stringify({
          sessionId: parsedAuth.sessionId
        })
      })

      if (response.ok) {
        const userData = await response.json()
        console.log('✅ Dados sincronizados com sucesso:', userData)

        // Salvar no localStorage
        localStorage.setItem('percolist_user', JSON.stringify(userData))

        // Atualizar Zustand
        setUser(userData)
        setAuthenticated(true)

        return true
      } else {
        console.error('❌ Erro na sincronização:', response.status)
        return false
      }
    } catch (err) {
      console.error('❌ Erro na sincronização:', err)
      return false
    }
  }

  // Função para popular Zustand a partir do localStorage
  const populateZustandFromStorage = () => {
    try {
      const authData = localStorage.getItem('percolist_auth')
      const userData = localStorage.getItem('percolist_user')

      if (authData) {
        const parsedAuth = JSON.parse(authData)
        setAuth(parsedAuth)
        setAuthenticated(true)
        console.log('✅ Auth data populado no Zustand:', parsedAuth)
      }

      if (userData) {
        const parsedUser = JSON.parse(userData)
        setUser(parsedUser)
        console.log('✅ User data populado no Zustand:', parsedUser)
      }
    } catch (err) {
      console.error('❌ Erro ao popular Zustand:', err)
    }
  }

  // Função para fazer logout completo
  const handleLogout = async () => {
    try {
      // 1. Limpar dados do backend (SQLite)
      await invoke('clear_saved_auth_data')

      // 2. Limpar dados do localStorage
      localStorage.removeItem('percolist_auth')
      localStorage.removeItem('percolist_user')

      // 3. Limpar Zustand
      clearUser()

      alert('✅ Logout realizado com sucesso!\n\nVocê foi desconectado e todos os dados foram removidos.')
    } catch (error) {
      console.error('❌ Erro no logout:', error)
      alert('❌ Erro ao fazer logout: ' + error)
    }
  }

  // Função para verificar se está logado
  const isAuthenticated = () => {
    const authData = localStorage.getItem('percolist_auth')
    return !!authData
  }

  // Função para salvar dados no localStorage (simulando recebimento)
  const saveToLocalStorage = (token: string, sessionId: string) => {
    try {
      const authData = {
        token,
        sessionId,
        authenticated: true
      }

      localStorage.setItem('percolist_auth', JSON.stringify(authData))
      console.log('✅ Dados salvos no localStorage:', authData)

      return true
    } catch (err) {
      console.error('❌ Erro ao salvar no localStorage:', err)
      return false
    }
  }

  // Função para carregar dados do localStorage
  const loadFromLocalStorage = () => {
    try {
      const authData = localStorage.getItem('percolist_auth')
      if (authData) {
        return JSON.parse(authData) as AuthData
      }
      return null
    } catch (err) {
      console.error('❌ Erro ao carregar do localStorage:', err)
      return null
    }
  }

  // Inicialização
  useEffect(() => {
    console.log('🚀 useAuthBlock - Inicializando...')
    setIsLoading(true)

    // Carregar dados salvos
    loadUserDataFromStorage()
    populateZustandFromStorage()

    // Sincronizar com API se necessário
    const authData = localStorage.getItem('percolist_auth')
    if (authData) {
      syncUserData().finally(() => {
        setIsLoading(false)
      })
    } else {
      setIsLoading(false)
    }
  }, [])

  // Escutar mudanças no localStorage
  useEffect(() => {
    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === 'percolist_auth') {
        console.log('🔄 percolist_auth mudou no localStorage')
        if (e.newValue) {
          try {
            const parsedAuth = JSON.parse(e.newValue)
            setAuth(parsedAuth)
            setAuthenticated(true)

            // Sincronizar dados
            syncUserData()
          } catch (err) {
            console.error('❌ Erro ao processar mudança no localStorage:', err)
          }
        } else {
          // Auth foi removido
          clearUser()
        }
      }
    }

    window.addEventListener('storage', handleStorageChange)
    return () => window.removeEventListener('storage', handleStorageChange)
  }, [setAuth, setAuthenticated, clearUser])

  return {
    // Estados
    isLoading,
    error,
    isBlocked,
    isAuthenticated: isAuthenticated(),

    // Funções
    syncUserData,
    handleLogout,
    saveToLocalStorage,
    loadFromLocalStorage,
    populateZustandFromStorage,

    // Dados
    user
  }
}
