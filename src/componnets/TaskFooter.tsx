import { useTaskStore } from "../store/task.store"
import { useState, useEffect, useMemo, useRef, useCallback } from "react"
import type { RefObject } from "react"
import { ClockIcon, MenuDefaultIcon, MenuSuccessIcon } from "../components"
// import { VolumeSlider } from "./VolumeSlider"
import { TaskButton } from "./TaskButton"
import Calendar from "./Calendar"
import { TaskList } from "./TaskList"
import { TaskListView } from "./TaskListView"
import { invoke } from "@tauri-apps/api/core"
// import { listen } from "@tauri-apps/api/event";

import { CalendarIcon } from "../components/CalendarIcon"
import { CalendarStartIcon } from "../components/CalendarStartIcon"

import { CirclePlusIcon } from "../components/CirclePlusIcon"
import { PlusSuccessIcon } from "../components/PlusSuccessIcon"
// import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCollapsedState, usePomodoroFlow } from "../hooks"
import { useQuery, useQueryClient } from "@tanstack/react-query"
import { formatTimeDisplay } from "../utils/format"
import { secondsToDuration } from "../utils/time"

interface TaskFooterProps {
  onAddClick: () => void
  buttonRef: RefObject<HTMLButtonElement | null>
  isModalOpen?: boolean
}

const teste = [
  {
    id: 73,
    task_id: 10,
    session_number: 1,
    session_type: "work",
    duration_seconds: 60,
    remaining_seconds: 60,
    status: "running",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 74,
    task_id: 10,
    session_number: 2,
    session_type: "mini_break",
    duration_seconds: 30,
    remaining_seconds: 30,
    status: "completed",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 75,
    task_id: 10,
    session_number: 3,
    session_type: "work",
    duration_seconds: 60,
    remaining_seconds: 60,
    status: "paused",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 76,
    task_id: 10,
    session_number: 4,
    session_type: "mini_break",
    duration_seconds: 30,
    remaining_seconds: 30,
    status: "pending",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 77,
    task_id: 10,
    session_number: 5,
    session_type: "work",
    duration_seconds: 60,
    remaining_seconds: 60,
    status: "pending",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 78,
    task_id: 10,
    session_number: 6,
    session_type: "mini_break",
    duration_seconds: 30,
    remaining_seconds: 30,
    status: "pending",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 79,
    task_id: 10,
    session_number: 7,
    session_type: "work",
    duration_seconds: 60,
    remaining_seconds: 60,
    status: "pending",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  },
  {
    id: 80,
    task_id: 10,
    session_number: 8,
    session_type: "long_break",
    duration_seconds: 60,
    remaining_seconds: 60,
    status: "pending",
    created_at: "2025-08-11T11:50:33.840169600+00:00"
  }
]

export function TaskFooter({ onAddClick, buttonRef, isModalOpen = false }: TaskFooterProps) {
  const {
    getTodayActiveTasks,
    getTodayActiveTasksWithSessions,
    tasks,
    tasksWithSessions,
    loadTasks,
    loadTasksWithSessions
  } = useTaskStore()
  const [currentTime, setCurrentTime] = useState(new Date())
  const [isCalendarOpen, setIsCalendarOpen] = useState(false)
  const [selectedDate, setSelectedDate] = useState<Date | null>(null)
  const [isTaskListViewOpen, setIsTaskListViewOpen] = useState(false)
  const anchorRefButtonListTask = useRef<HTMLButtonElement | null>(null)
  const taskListViewRef = useRef<HTMLDivElement | null>(null)
  const { isCollapsed } = useCollapsedState()
  const [isDrawerOpen, setIsDrawerOpen] = useState(false)

  const queryClient = useQueryClient()
  // const { screenWidth } = useScreenWidth()
  const disabledRef = useRef(false)

  // Otimizar cálculos com useMemo para evitar recálculos desnecessários
  const todayTasks = useMemo(() => {
    const todayTasksWithSessions = getTodayActiveTasksWithSessions()
    return todayTasksWithSessions.length > 0 ? todayTasksWithSessions : getTodayActiveTasks()
  }, [getTodayActiveTasksWithSessions, getTodayActiveTasks, tasksWithSessions, tasks])

  const [orderedTasks, setOrderedTasks] = useState(todayTasks)

  useEffect(() => {
    setOrderedTasks(todayTasks)
  }, [todayTasks])

  const allTasks = useMemo(() => {
    return tasksWithSessions.length > 0 ? tasksWithSessions : tasks
  }, [tasksWithSessions, tasks])

  const datesWithTasks = useMemo(() => {
    return allTasks.map(task => {
      const [year, month, day] = task.scheduled_date.split("-").map(Number)
      return new Date(year, month - 1, day)
    })
  }, [allTasks])

  const getTasksForDate = useMemo(() => {
    return (date: Date) => {
      const dateStr = date.toISOString().split("T")[0]
      return allTasks.filter(task => task.scheduled_date === dateStr)
    }
  }, [allTasks])

  // Remover o timer que atualiza currentTime a cada segundo - estava causando re-renderizações desnecessárias
  // useEffect(() => {
  //   const timer = setInterval(() => {
  //     setCurrentTime(new Date())
  //   }, 1000)
  //   return () => clearInterval(timer)
  // }, [])

  const handleDaySelect = (date: Date | undefined) => {
    if (date) {
      const dateStr = date.toISOString().split("T")[0]
      const tasksForDate = allTasks.filter(task => task.scheduled_date === dateStr)

      if (tasksForDate.length > 0) {
        setSelectedDate(date)
      } else {
        setSelectedDate(null)
        setCurrentTime(date)
        setIsCalendarOpen(false)
      }
    }
  }

  const toggleCalendar = async () => {
    if (!isCalendarOpen) {
      await invoke("expand_window_for_modal")
    } else {
      await invoke("reset_window_size")
    }
    setIsCalendarOpen(!isCalendarOpen)
    setSelectedDate(null)
  }

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        isTaskListViewOpen &&
        anchorRefButtonListTask.current &&
        !anchorRefButtonListTask.current.contains(event.target as Node) &&
        taskListViewRef.current &&
        !taskListViewRef.current.contains(event.target as Node)
      ) {
        setIsTaskListViewOpen(false)
        invoke("reset_window_size")
      }
    }

    document.addEventListener("mousedown", handleClickOutside)
    return () => {
      document.removeEventListener("mousedown", handleClickOutside)
    }
  }, [isTaskListViewOpen])

  useEffect(() => {
    const handleClickOutside = async (event: MouseEvent) => {
      const target = event.target as HTMLElement
      const isClickInside =
        target.closest(".calendar-container") || target.closest(".calendar-trigger")
      const isClickOnInput =
        target.tagName.toLowerCase() === "input" || target.tagName.toLowerCase() === "button"

      if (!isClickInside && !isClickOnInput) {
        setIsCalendarOpen(false)
        setSelectedDate(null)
      }
    }

    document.addEventListener("mousedown", handleClickOutside)
    return () => {
      document.removeEventListener("mousedown", handleClickOutside)
    }
  }, [])

  // Observar mudanças no estado do calendário para controlar o resize
  useEffect(() => {
    if (!isCalendarOpen) {
      invoke("reset_window_size")
    }
  }, [isCalendarOpen])

  // Função para recarregar dados das tarefas
  const handleTaskAction = async (taskId: string, action: "start" | "pause") => {
    console.log(`🔄 TaskFooter - Recarregando dados após ação ${action} na tarefa ${taskId}`)
    try {
      // Recarregar dados do backend
      await Promise.all([loadTasks(), loadTasksWithSessions()])
      console.log("✅ TaskFooter - Dados recarregados com sucesso")
    } catch (error) {
      console.error("❌ TaskFooter - Erro ao recarregar dados:", error)
    }
  }
  useEffect(() => {
    if (isDrawerOpen) return
    const interval = setInterval(async () => {
      try {
        if (!isCollapsed) {
          await invoke("disable_click_through")
          disabledRef.current = false
          return
        }

        if (disabledRef.current) {
          await invoke("enable_click_through")
        } else {
          await invoke("disable_click_through")
        }

        disabledRef.current = !disabledRef.current
      } catch (err) {
        console.error("Erro ao alternar click-through:", err)
      }
    }, 500)

    return () => clearInterval(interval)
  }, [isCollapsed, isDrawerOpen])

  const { data: activeTaskId } = useQuery<number | null>({
    queryKey: ["active-task"],
    queryFn: async () => {
      const activeTaskId = await invoke<number | null>("get_active_task_id")

      console.log("activeTaskId", activeTaskId)
      return activeTaskId
    }
  })

  const { data: pomodoroSessionsData = [] } = useQuery({
    queryKey: ["active-pomodoro-sessions", activeTaskId],
    queryFn: async () => {
      if (!activeTaskId) return []
      const rows = await invoke("get_pomodoro_sessions_by_task", { taskId: activeTaskId })
      return Array.isArray(rows) ? rows : []
    },
    enabled: !!activeTaskId
  })

  const { sessions, currentSession } = usePomodoroFlow(pomodoroSessionsData, activeTaskId)
  useEffect(() => {
    console.log("pomodoroSessionsData", pomodoroSessionsData)
  }, [JSON.stringify(pomodoroSessionsData)])

  // Usar o hook usePomodoroFlow com as sessões carregadas
  useEffect(() => {
    console.log("activeTaskIsssss", sessions)
  }, [JSON.stringify(sessions)])

  // console.log("sessions", sessions)

  const pomodoroCycles =
    sessions?.length > 0 ? sessions?.filter((session: any) => session.session_type === "work") : []
  const pomodoroCyclesCompleted =
    pomodoroCycles?.length > 0
      ? pomodoroCycles?.filter((session: any) => session.status === "completed")
      : []

  return (
    <div style={{ padding: "0px 16px" }} className="w-full bg-black text-white  ">
      <div className="h-[55px] flex items-center relative z-10 justify-between px-4 backdrop-blur-sm border-t border-[#7F7F7F]">
        {/* Tasks Section */}
        <div className="flex items-center gap-3 flex-1 overflow-hidden">
          <div className="flex gap-2 overflow-x-auto scrollbar-hide">
            <div className="w-44 h-auto   gap-1 flex items-center justify-between">
              <div className="flex flex-col items-center justify-center w-4/6">
                <span className="text-[10px] font-bold text-white">POMODORO</span>
                <div
                  style={{
                    padding: "4px"
                  }}
                  className="select-none flex items-center gap-1 bg-[#444444] w-full rounded-full"
                >
                  <div className="w-6 h-6 flex items-center justify-center bg-[#17FF8B] rounded-full">
                    <ClockIcon className="w-6 h-6 text-white" />
                  </div>
                  <span className="text-[10px] font-bold text-white">
                    {pomodoroCyclesCompleted.length}/{pomodoroCycles.length}
                  </span>
                  <div className="text-[10px] font-bold text-white">|</div>
                  <span className="text-[10px] font-bold text-[#17FF8B]">
                    {formatTimeDisplay(secondsToDuration(currentSession?.remaining_seconds || 0))}
                  </span>
                </div>
              </div>
              <div className="w-2/6  flex flex-col items-center justify-center">
                <span className="text-[10px] text-white font-bold">TARREFAS</span>
                <div
                  style={{
                    padding: "4px"
                  }}
                  className="select-none flex items-center gap-1 bg-[#444444] w-full rounded-full h-8 justify-center"
                >
                  <span className="text-[10px] text-white font-bold flex items-center gap-1">
                    <span className="text-[#17FF8B]">12</span> | 100
                  </span>
                </div>
              </div>
            </div>
            {orderedTasks.length === 0 ? (
              <div className="text-gray-400 text-center flex items-center justify-center">
                Nenhuma tarefa para hoje
              </div>
            ) : (
              <div className="flex gap-2 translate-y-2">
                {orderedTasks.slice(0, 5).map((task, index) => (
                  <TaskButton
                    key={task.id}
                    task={task}
                    index={index}
                    onDragAction={handleTaskAction}
                  />
                ))}
                {orderedTasks.length > 5 && (
                  <div className="flex items-center justify-center min-w-[60px] text-white rounded-lg text-xl font-bold">
                    +{orderedTasks.length - 5}
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        {/* System Section with Add Button */}
        <div className="flex items-center gap-4 flex-shrink-0">
          {/* Pomodoro Controls */}

          {/* Add Button */}
          <button
            ref={buttonRef}
            onClick={onAddClick}
            className="flex items-center cursor-pointer bg-[#444444] rounded-full gap-2 text-sm font-medium text-zinc-300 hover:text-zinc-100 hover:bg-white/10 transition-colors w-8 h-8 justify-center"
            title="Adicionar nova tarefa"
          >
            {isModalOpen ? (
              <PlusSuccessIcon className="w-4 h-4 text-[#17FF8B]" />
            ) : (
              <CirclePlusIcon className="w-4 h-4" />
            )}
          </button>

          <div>
            {currentSession?.remaining_seconds}
            {currentSession?.session_type}
            {currentSession?.session_number}
          </div>
          {/* Task List View Button */}
          {/* Menu de tasks */}
          <button
            ref={anchorRefButtonListTask}
            onClick={async () => {
              await invoke("expand_window_for_modal")
              setIsTaskListViewOpen(!isTaskListViewOpen)
            }}
            className="flex items-center cursor-pointer bg-[#444444] p-2 w-8 h-8  justify-center rounded-full gap-2 text-sm font-medium text-zinc-300 hover:text-zinc-100 hover:bg-white/10 transition-colors  relative"
            title="Ver todas as tarefas"
          >
            <div className="w-4 h-4 flex items-center justify-center">
              <div
                className={`absolute inset-0 flex items-center justify-center transition-all duration-300 ease-in-out ${
                  isTaskListViewOpen
                    ? "opacity-0 scale-90 rotate-12"
                    : "opacity-100 scale-100 rotate-0"
                }`}
              >
                <MenuDefaultIcon className="w-4 h-4 text-zinc-300" />
              </div>

              <div
                className={`absolute inset-0 flex items-center justify-center transition-all duration-300 ease-in-out ${
                  isTaskListViewOpen
                    ? "opacity-100 scale-100 rotate-0"
                    : "opacity-0 scale-90 -rotate-12"
                }`}
              >
                <MenuSuccessIcon className="w-4 h-4 text-[#17FF8B]" />
              </div>
            </div>
          </button>

          {/* Volume Control with Slider */}
          {/* <VolumeSlider /> */}

          {/* Date and Time */}
          <div
            style={{ padding: "0px 10px" }}
            className="flex items-center justify-center bg-[#444444] rounded-full h-8 gap-3 text-sm"
          >
            <div className="relative">
              <div
                className="flex items-center gap-1 cursor-pointer calendar-trigger"
                onClick={toggleCalendar}
              >
                <div className="relative w-6 h-6  select-none">
                  <div
                    className={`absolute inset-0 transition-all duration-500 ${
                      isCalendarOpen ? "opacity-100 scale-100" : "opacity-0 scale-90"
                    }`}
                  >
                    <CalendarStartIcon className="w-6 h-6 text-[#17FF8B]" />
                  </div>
                  <div
                    className={`absolute inset-0 transition-all duration-500 ${
                      !isCalendarOpen ? "opacity-100 scale-100" : "opacity-0 scale-90"
                    }`}
                  >
                    <CalendarIcon className="w-6 h-6 text-white" />
                  </div>
                </div>
                <span className=" select-none">
                  {currentTime.toLocaleDateString("pt-BR", {
                    day: "2-digit",
                    month: "2-digit",
                    year: "numeric"
                  })}
                </span>
              </div>

              {isCalendarOpen && (
                <div
                  className="absolute select-none right-[calc(100%-80px)] top-[calc(100%+8px)] z-50 calendar-container"
                  onClick={e => e.stopPropagation()}
                >
                  {selectedDate ? (
                    <TaskList
                      tasks={getTasksForDate(selectedDate)}
                      date={selectedDate}
                      onBack={() => setSelectedDate(null)}
                      onClose={() => {
                        setIsCalendarOpen(false)
                        setSelectedDate(null)
                      }}
                    />
                  ) : (
                    <Calendar
                      selected={currentTime}
                      onSelect={handleDaySelect}
                      highlightedDates={datesWithTasks}
                      onUnmount={
                        !isCalendarOpen
                          ? () => {
                              setIsCalendarOpen(false)
                              setSelectedDate(null)
                            }
                          : undefined
                      }
                    />
                  )}
                </div>
              )}
            </div>
            <span className="border-x border-[#7F7F7F] h-6 "></span>
            <div className="flex items-center gap-1 selected-none">
              <span className="font-medium select-none text-xs">
                {currentTime.toLocaleDateString("pt-BR", { weekday: "short" }).toUpperCase()}{" "}
                {currentTime.toLocaleTimeString("pt-BR", {
                  hour: "2-digit",
                  minute: "2-digit"
                })}
              </span>
            </div>
          </div>
        </div>
        <div
          id="myDiv"
          onClick={async () => {
            if (isDrawerOpen) {
              await invoke("reset_window_size")
              await invoke("enable_click_through")
              setIsDrawerOpen(false)
            } else {
              setIsDrawerOpen(true)
              await invoke("disable_click_through")
              await invoke("expand_window_for_modal")
            }
          }}
          className="select-none absolute top-[45px] left-1/2 transform -translate-x-1/2
                  bg-black text-white p-4 h-5 w-16 z-50 rounded-lg
                  flex items-center justify-center text-xs cursor-pointer hover:bg-red-800"
        >
          {isDrawerOpen ? "Fechar" : "Abrir"}
        </div>
      </div>

      {/* Task List View Modal */}
      <TaskListView
        ref={taskListViewRef}
        anchorRef={anchorRefButtonListTask}
        isOpen={isTaskListViewOpen}
        onClose={async () => {
          setIsTaskListViewOpen(false)
          await invoke("reset_window_size")
        }}
      />
    </div>
  )
}
