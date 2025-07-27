import { create } from 'zustand'

interface User {
  id: string
  email: string
  name: string
  avatar?: string
}

interface AuthData {
  token: string
  sessionId: string
  authenticated: boolean
}

interface UserState {
  user: User | null
  auth: AuthData | null
  isAuthenticated: boolean
  isBlocked: boolean
  setUser: (user: User | null) => void
  setAuth: (auth: AuthData | null) => void
  setAuthenticated: (authenticated: boolean) => void
  setBlocked: (blocked: boolean) => void
  clearUser: () => void
}

export const useUserStore = create<UserState>((set) => ({
  user: null,
  auth: null,
  isAuthenticated: false,
  isBlocked: false,

  setUser: (user) => set({ user }),
  setAuth: (auth) => set({ auth }),
  setAuthenticated: (authenticated) => set({ isAuthenticated: authenticated }),
  setBlocked: (blocked) => set({ isBlocked: blocked }),

  clearUser: () => set({
    user: null,
    auth: null,
    isAuthenticated: false,
    isBlocked: false
  })
}))
