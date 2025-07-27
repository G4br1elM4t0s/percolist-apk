import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { api } from "../lib/api";

type SyncResponse = {
  active: boolean
  plan: string
  trial: boolean
}

export const useAuthLockAndRelease = () => {
  const [enabled, setEnabled] = useState(false)

  const token = localStorage.getItem('token')
  const sessionId = localStorage.getItem('sessionId')

  useEffect(() => {
    if (!token || !sessionId) {
      console.warn('🔒 Token ou sessionId ausente. Redirecionando para login.')
    } else {
      setEnabled(true)
    }
  }, [token, sessionId])

  const {
    data,
    error,
    isFetching,
  } = useQuery<SyncResponse, any>({
    queryKey: ['auth-sync'],
    queryFn: async () => {
      const response = await api.post('/auth/sync', {
        token,
        sessionId,
      })
      return response.data
    },
    enabled,
    refetchInterval: 5 * 60 * 1000,
    retry: false,
  })

  useEffect(() => {
    if (error) {
      if (error?.response?.status === 500) {
        console.warn('⚠️ Erro 500 ao validar sessão. Ignorando.')
      } else {
        console.error('❌ Sessão inválida:', error)
      }
    }
  }, [!!error])

  return {
    isChecking: isFetching,
    authData: data,
    hasAccess: !!data?.active,
    error,
  }
}
