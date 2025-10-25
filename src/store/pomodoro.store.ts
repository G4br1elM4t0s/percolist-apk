import { create } from "zustand"
import { invoke } from "@tauri-apps/api/core"

export type PomodoroConfig = {
  workDuration: number // em minutos
  shortBreakDuration: number // em minutos
  longBreakDuration: number // em minutos
  totalCycles: number
}

export type PomodoroState = {
  taskId: string
  currentCycle: number
  totalCycles: number
  currentTime: number // tempo restante em segundos
  isRunning: boolean
  isPaused: boolean
  sessionType: "work" | "short_break" | "long_break"
  totalWorkedMinutes: number
  completedCycles: number
}

export type PomodoroStore = {
  // Estado atual
  activePomodoros: Map<string, PomodoroState>
  config: PomodoroConfig
  
  // Ações
  initializePomodoro: (taskId: string) => Promise<void>
  startPomodoro: (taskId: string) => Promise<void>
  pausePomodoro: (taskId: string) => Promise<void>
  resumePomodoro: (taskId: string) => Promise<void>
  resetPomodoro: (taskId: string) => Promise<void>
  skipBreak: (taskId: string) => Promise<void>
  
  // Getters
  getPomodoroState: (taskId: string) => PomodoroState | null
  getFormattedTime: (taskId: string) => string
  getTotalWorkedTime: (taskId: string) => number
  
  // Configuração
  updateConfig: (config: Partial<PomodoroConfig>) => void
  
  // Tick do timer
  tick: () => void
}

const DEFAULT_CONFIG: PomodoroConfig = {
  workDuration: 25,
  shortBreakDuration: 5,
  longBreakDuration: 15,
  totalCycles: 4
}

export const usePomodoroStore = create<PomodoroStore>((set, get) => ({
  activePomodoros: new Map(),
  config: DEFAULT_CONFIG,

  initializePomodoro: async (taskId: string) => {
    try {
      // Verificar se já existe um Pomodoro para esta tarefa
      const existingState = await invoke<PomodoroState | null>("get_pomodoro_state", { taskId })
      
      if (existingState) {
        // Restaurar estado existente
        const activePomodoros = new Map(get().activePomodoros)
        activePomodoros.set(taskId, existingState)
        set({ activePomodoros })
      } else {
        // Criar novo Pomodoro
        const config = get().config
        const newState: PomodoroState = {
          taskId,
          currentCycle: 1,
          totalCycles: config.totalCycles,
          currentTime: config.workDuration * 60, // converter para segundos
          isRunning: false,
          isPaused: false,
          sessionType: "work",
          totalWorkedMinutes: 0,
          completedCycles: 0
        }
        
        // Salvar no backend
        await invoke("save_pomodoro_state", { taskId, state: newState })
        
        // Atualizar store local
        const activePomodoros = new Map(get().activePomodoros)
        activePomodoros.set(taskId, newState)
        set({ activePomodoros })
      }
    } catch (error) {
      console.error("Erro ao inicializar Pomodoro:", error)
    }
  },

  startPomodoro: async (taskId: string) => {
    const state = get().activePomodoros.get(taskId)
    if (!state) return

    const updatedState = {
      ...state,
      isRunning: true,
      isPaused: false
    }

    try {
      await invoke("save_pomodoro_state", { taskId, state: updatedState })
      
      const activePomodoros = new Map(get().activePomodoros)
      activePomodoros.set(taskId, updatedState)
      set({ activePomodoros })
    } catch (error) {
      console.error("Erro ao iniciar Pomodoro:", error)
    }
  },

  pausePomodoro: async (taskId: string) => {
    const state = get().activePomodoros.get(taskId)
    if (!state) return

    const updatedState = {
      ...state,
      isRunning: false,
      isPaused: true
    }

    try {
      await invoke("save_pomodoro_state", { taskId, state: updatedState })
      
      const activePomodoros = new Map(get().activePomodoros)
      activePomodoros.set(taskId, updatedState)
      set({ activePomodoros })
    } catch (error) {
      console.error("Erro ao pausar Pomodoro:", error)
    }
  },

  resumePomodoro: async (taskId: string) => {
    const state = get().activePomodoros.get(taskId)
    if (!state) return

    const updatedState = {
      ...state,
      isRunning: true,
      isPaused: false
    }

    try {
      await invoke("save_pomodoro_state", { taskId, state: updatedState })
      
      const activePomodoros = new Map(get().activePomodoros)
      activePomodoros.set(taskId, updatedState)
      set({ activePomodoros })
    } catch (error) {
      console.error("Erro ao retomar Pomodoro:", error)
    }
  },

  resetPomodoro: async (taskId: string) => {
    const config = get().config
    const resetState: PomodoroState = {
      taskId,
      currentCycle: 1,
      totalCycles: config.totalCycles,
      currentTime: config.workDuration * 60,
      isRunning: false,
      isPaused: false,
      sessionType: "work",
      totalWorkedMinutes: 0,
      completedCycles: 0
    }

    try {
      await invoke("save_pomodoro_state", { taskId, state: resetState })
      
      const activePomodoros = new Map(get().activePomodoros)
      activePomodoros.set(taskId, resetState)
      set({ activePomodoros })
    } catch (error) {
      console.error("Erro ao resetar Pomodoro:", error)
    }
  },

  skipBreak: async (taskId: string) => {
    const state = get().activePomodoros.get(taskId)
    if (!state || state.sessionType === "work") return

    const config = get().config
    const nextCycle = state.currentCycle + (state.sessionType === "long_break" ? 1 : 0)
    
    const updatedState = {
      ...state,
      currentCycle: nextCycle,
      currentTime: config.workDuration * 60,
      sessionType: "work" as const,
      isRunning: false,
      isPaused: false
    }

    try {
      await invoke("save_pomodoro_state", { taskId, state: updatedState })
      
      const activePomodoros = new Map(get().activePomodoros)
      activePomodoros.set(taskId, updatedState)
      set({ activePomodoros })
    } catch (error) {
      console.error("Erro ao pular pausa:", error)
    }
  },

  getPomodoroState: (taskId: string) => {
    return get().activePomodoros.get(taskId) || null
  },

  getFormattedTime: (taskId: string) => {
    const state = get().activePomodoros.get(taskId)
    if (!state) return "00:00"

    const minutes = Math.floor(state.currentTime / 60)
    const seconds = state.currentTime % 60
    return `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
  },

  getTotalWorkedTime: (taskId: string) => {
    const state = get().activePomodoros.get(taskId)
    return state?.totalWorkedMinutes || 0
  },

  updateConfig: (newConfig: Partial<PomodoroConfig>) => {
    const config = { ...get().config, ...newConfig }
    set({ config })
    
    // Salvar configuração no localStorage
    localStorage.setItem('pomodoro-config', JSON.stringify(config))
  },

  tick: () => {
    const activePomodoros = new Map(get().activePomodoros)
    const config = get().config
    let hasChanges = false

    for (const [taskId, state] of activePomodoros) {
      if (!state.isRunning) continue

      const newTime = Math.max(0, state.currentTime - 1)
      let updatedState = { ...state, currentTime: newTime }

      // Se o tempo acabou
      if (newTime === 0) {
        if (state.sessionType === "work") {
          // Completou um ciclo de trabalho
          const newCompletedCycles = state.completedCycles + 1
          const newTotalWorkedMinutes = state.totalWorkedMinutes + config.workDuration

          if (newCompletedCycles >= config.totalCycles) {
            // Todos os ciclos completados - pausa longa
            updatedState = {
              ...updatedState,
              sessionType: "long_break",
              currentTime: config.longBreakDuration * 60,
              completedCycles: newCompletedCycles,
              totalWorkedMinutes: newTotalWorkedMinutes,
              isRunning: false
            }
          } else {
            // Pausa curta
            updatedState = {
              ...updatedState,
              sessionType: "short_break",
              currentTime: config.shortBreakDuration * 60,
              completedCycles: newCompletedCycles,
              totalWorkedMinutes: newTotalWorkedMinutes,
              isRunning: false
            }
          }
        } else {
          // Completou uma pausa
          if (state.sessionType === "long_break") {
            // Resetar para novo ciclo completo
            updatedState = {
              ...updatedState,
              currentCycle: 1,
              sessionType: "work",
              currentTime: config.workDuration * 60,
              completedCycles: 0,
              isRunning: false
            }
          } else {
            // Próximo ciclo de trabalho
            updatedState = {
              ...updatedState,
              currentCycle: state.currentCycle + 1,
              sessionType: "work",
              currentTime: config.workDuration * 60,
              isRunning: false
            }
          }
        }

        // Salvar estado atualizado no backend
        invoke("save_pomodoro_state", { taskId, state: updatedState }).catch(console.error)
      }

      if (updatedState !== state) {
        activePomodoros.set(taskId, updatedState)
        hasChanges = true
      }
    }

    if (hasChanges) {
      set({ activePomodoros })
    }
  }
}))

// Inicializar configuração do localStorage
const savedConfig = localStorage.getItem('pomodoro-config')
if (savedConfig) {
  try {
    const config = JSON.parse(savedConfig)
    usePomodoroStore.getState().updateConfig(config)
  } catch (error) {
    console.error("Erro ao carregar configuração do Pomodoro:", error)
  }
}