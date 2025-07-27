import axios from 'axios'

export const api = axios.create({
  baseURL: 'http://localhost:3000',
  timeout: 10000,
})

// Interceptor para adicionar token automaticamente
api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// Interceptor para tratar erros
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      // Token expirado ou inválido
      localStorage.removeItem('token')
      localStorage.removeItem('sessionId')
      window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)
