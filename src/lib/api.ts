import axios from 'axios'

// Contador global para rastrear chamadas da API
let apiCallCount = 0;

export const getApiCallCount = () => apiCallCount;
export const resetApiCallCount = () => { apiCallCount = 0; };

export const api = axios.create({
  baseURL: 'https://api.percolist.com.br',
  timeout: 10000,
})

// Interceptor para adicionar token automaticamente e contar chamadas
api.interceptors.request.use((config) => {
  // Incrementar contador
  apiCallCount++;
  console.log(`🔢 API Call #${apiCallCount} - ${config.method?.toUpperCase()} ${config.url}`);

  const token = localStorage.getItem('token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// Interceptor para tratar erros
api.interceptors.response.use(
  (response) => {
    console.log(`✅ API Call #${apiCallCount} - Success: ${response.status}`);
    return response
  },
  (error) => {
    console.log(`❌ API Call #${apiCallCount} - Error: ${error.response?.status || 'Network Error'}`);
    if (error.response?.status === 401) {
      // Token expirado ou inválido
      localStorage.removeItem('token')
      localStorage.removeItem('sessionId')
      window.location.href = '/login'
    }
    return Promise.reject(error)
  }
)
