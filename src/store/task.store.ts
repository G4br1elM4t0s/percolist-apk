import { create } from "zustand"
import { invoke } from "@tauri-apps/api/core"

export type Task = {
  id?: string
  name: string
  description?: string
  user: string
  estimated_hours: number
  worked_hours: number
  scheduled_date: string
  end_date?: string | null
  status: "pending" | "in_progress" | "paused" | "waiting" | "completed"
  created_at: string
  started_at: string | null
  completed_at: string | null
  should_count: boolean
  count_value: number
}

export type ActiveSessionInfo = {
  session_type: "work" | "break"
  started_at: string
  ends_at: string
  duration_seconds: number
}

export type PomodoroSessionInfo = {
  id?: number
  session_number: number
  session_type: "work" | "break"
  duration_seconds: number
  created_at: string
  is_active: boolean
  started_at?: string | null
}

export type TaskWithActiveSession = Task & {
  active_session?: ActiveSessionInfo | null
  pomodoro_sessions: PomodoroSessionInfo[]
}

type TaskStore = {
  tasks: Task[]
  tasksWithSessions: TaskWithActiveSession[]
  loadTasks: () => Promise<void>
  loadTasksWithSessions: () => Promise<void>
  addTask: (task: Task) => Promise<void>
  updateTask: (taskId: string, task: Partial<Task>) => Promise<void>
  incrementTaskCount: (taskId: string) => Promise<number>
  rescheduleTasks: () => Promise<number[]>
  startTask: (taskId: string) => Promise<void>
  startTaskWithoutStopping: (taskId: string) => Promise<void>
  pauseTask: (taskId: string) => Promise<void>
  resumeTask: (taskId: string) => Promise<void>
  completeTask: (taskId: string) => Promise<void>
  deleteTask: (taskId: string) => Promise<void>
  getTaskRemainingTime: (taskId: string) => Promise<number>
  getTaskTotalWorkedTime: (taskId: string) => Promise<number>
  checkPomodoroSessions: () => Promise<number[]>
  getTodayTasks: () => Task[]
  getTodayTasksWithSessions: () => TaskWithActiveSession[]
  getTodayActiveTasks: () => Task[]
  getTodayActiveTasksWithSessions: () => TaskWithActiveSession[]
  hasActiveTask: () => boolean
  getActiveTask: () => Task | TaskWithActiveSession | null
  getTasksByStatus: (status: Task["status"]) => Task[]
  getTasksWithSessionsByStatus: (status: Task["status"]) => TaskWithActiveSession[]
  getPendingTasks: () => Task[]
  getCompletedTasks: () => Task[]
  getInProgressTasks: () => Task[]
  getPausedTasks: () => Task[]
  getAllTasksByStatus: () => Record<Task["status"], Task[]>
}

export const useTaskStore = create<TaskStore>((set, get) => ({
  tasks: [],
  tasksWithSessions: [],

  loadTasks: async () => {
    try {
      const tasks = await invoke<Task[]>("load_tasks")

      set({ tasks })
    } catch (error) {
      console.error("Error loading tasks:", error)
    }
  },

  addTask: async (task: Task) => {
    try {
      // Mapear os dados do formulário para o formato que o Rust espera
      const params = {
        name: task.name,
        description: task.description || null,
        user: task.user,
        estimatedHours: task.estimated_hours,
        scheduledDate: task.scheduled_date,
        endDate: task.end_date || null,
        shouldCount: task.should_count,
        countValue: task.count_value,
        pomodoroCycles: 4
      }

      const newTask = await invoke<Task>("add_task", params)
      console.log("Task adicionada com sucesso:", newTask)

      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Erro detalhado ao chamar add_task no Rust:", error)
      throw error
    }
  },

  updateTask: async (taskId: string, task: Partial<Task>) => {
    try {
      const params = {
        taskId,
        name: task.name!,
        description: task.description || null,
        estimatedHours: task.estimated_hours!,
        workedHours: task.worked_hours!,
        scheduledDate: task.scheduled_date!,
        endDate: task.end_date || null,
        shouldCount: task.should_count!
      }

      await invoke("update_task", params)

      // Recarregar ambos os tipos de dados
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error updating task:", error)
      throw error
    }
  },

  incrementTaskCount: async (taskId: string) => {
    try {
      const newCount = await invoke<number>("increment_task_count", { taskId })

      // Recarregar ambos os tipos de dados
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])

      return newCount
    } catch (error) {
      console.error("Error incrementing task count:", error)
      throw error
    }
  },

  rescheduleTasks: async () => {
    try {
      const rescheduledTaskIds = await invoke<number[]>("reschedule_tasks")

      // Recarregar ambos os tipos de dados após remanejamento
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])

      console.log(`🔄 ${rescheduledTaskIds.length} tarefas remanejadas:`, rescheduledTaskIds)
      return rescheduledTaskIds
    } catch (error) {
      console.error("Error rescheduling tasks:", error)
      throw error
    }
  },

  startTask: async (taskId: string) => {
    try {
      await invoke("start_task", {
        taskId,
        stopAndStart: true
      })
      // Recarregar ambos os tipos de dados para obter status atualizado
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error starting task:", error)
    }
  },

  startTaskWithoutStopping: async (taskId: string) => {
    try {
      await invoke("start_task", {
        taskId,
        stopAndStart: false
      })
      // Recarregar ambos os tipos de dados para obter status atualizado
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error starting task without stopping others:", error)
    }
  },

  pauseTask: async (taskId: string) => {
    try {
      await invoke("pause_task", { taskId })
      // Recarregar ambos os tipos de dados para obter status atualizado
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error pausing task:", error)
    }
  },

  resumeTask: async (taskId: string) => {
    try {
      await invoke("resume_task", { taskId })
      // Recarregar ambos os tipos de dados para obter status atualizado
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error resuming task:", error)
    }
  },

  completeTask: async (taskId: string) => {
    try {
      await invoke("complete_task", { taskId })
      // Recarregar ambos os tipos de dados para obter status atualizado
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error completing task:", error)
    }
  },

  deleteTask: async (taskId: string) => {
    try {
      await invoke("delete_task", { taskId })
      // Recarregar ambos os tipos de dados
      await Promise.all([get().loadTasks(), get().loadTasksWithSessions()])
    } catch (error) {
      console.error("Error deleting task:", error)
    }
  },

  getTaskRemainingTime: async (taskId: string) => {
    try {
      const remainingSeconds = await invoke<number>("get_task_remaining_time", { taskId })
      return remainingSeconds
    } catch (error) {
      console.error("Error getting task remaining time:", error)
      return 0
    }
  },

  getTaskTotalWorkedTime: async (taskId: string) => {
    try {
      const totalSeconds = await invoke<number>("get_task_total_worked_time", { taskId: parseInt(taskId) })
      return totalSeconds
    } catch (error) {
      console.error("Error getting task total worked time:", error)
      throw error
    }
  },

  loadTasksWithSessions: async () => {
    try {
      const tasksWithSessions = await invoke<TaskWithActiveSession[]>("load_tasks_with_sessions")
      set({ tasksWithSessions })
    } catch (error) {
      console.error("Error loading tasks with sessions:", error)
    }
  },

  checkPomodoroSessions: async () => {
    try {
      const advancedTasks = await invoke<number[]>("check_pomodoro_sessions")

      // Nota: O recarregamento é feito pelo usePomodoroChecker para evitar duplicação
      // quando chamado automaticamente vs manualmente

      return advancedTasks
    } catch (error) {
      console.error("Error checking pomodoro sessions:", error)
      return []
    }
  },

  getTodayTasks: () => {
    const today = new Date().toISOString().split("T")[0]
    const todayTasks = get().tasks.filter(task => task.scheduled_date === today)
    return todayTasks
  },

  getTodayTasksWithSessions: () => {
    const today = new Date().toISOString().split("T")[0]
    const todayTasks = get().tasksWithSessions.filter(task => task.scheduled_date === today)
    return todayTasks
  },

  getTodayActiveTasks: () => {
    const today = new Date().toISOString().split("T")[0]
    const todayTasks = get().tasks.filter(
      task => task.scheduled_date === today && task.status !== "completed"
    )
    return todayTasks
  },

  getTodayActiveTasksWithSessions: () => {
    const today = new Date().toISOString().split("T")[0]
    const todayTasks = get().tasksWithSessions.filter(
      task => task.scheduled_date === today && task.status !== "completed"
    )
    return todayTasks
  },

  hasActiveTask: () => {
    const allTasks = [...get().tasks, ...get().tasksWithSessions]
    return allTasks.some(task => task.status === "in_progress" || task.status === "waiting")
  },

  getActiveTask: () => {
    const allTasks = [...get().tasks, ...get().tasksWithSessions]
    return allTasks.find(task => task.status === "in_progress" || task.status === "waiting") || null
  },

  getTasksByStatus: (status: Task["status"]) => {
    const tasks = get().tasks.filter(task => task.status === status)
    return tasks
  },

  getTasksWithSessionsByStatus: (status: Task["status"]) => {
    const tasksWithSessions = get().tasksWithSessions.filter(task => task.status === status)
    return tasksWithSessions
  },

  getPendingTasks: () => {
    const pendingTasks = get().tasks.filter(task => task.status === "pending")
    return pendingTasks
  },

  getCompletedTasks: () => {
    const completedTasks = get().tasks.filter(task => task.status === "completed")
    return completedTasks
  },

  getInProgressTasks: () => {
    const inProgressTasks = get().tasks.filter(task => task.status === "in_progress")
    return inProgressTasks
  },

  getPausedTasks: () => {
    const pausedTasks = get().tasks.filter(task => task.status === "paused")
    return pausedTasks
  },

  getAllTasksByStatus: () => {
    const statuses: Task["status"][] = ["pending", "in_progress", "paused", "waiting", "completed"]
    const tasksByStatus = statuses.reduce((acc, status) => {
      acc[status] = get().tasks.filter(task => task.status === status)
      return acc
    }, {} as Record<Task["status"], Task[]>)
    return tasksByStatus
  }
}))
