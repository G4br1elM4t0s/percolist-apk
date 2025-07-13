import { invoke } from "@tauri-apps/api/core"
import { useEffect, useState, useRef } from "react"
import type { Task, TaskWithActiveSession } from "../store/task.store"
import { formatTimeDisplay } from "../utils/format"
import { calculateTimeRemaining, secondsToDuration } from "../utils/time"
import { SettingsIcon } from "../components/SettingsIcon"
import { useTaskStore } from "../store/task.store"
import { TaskEditModal } from "./TaskEditModal"

interface TaskButtonProps {
  task: Task | TaskWithActiveSession
  index: number
  onDragAction?: (taskId: string, action: "start" | "pause") => void
  isListView?: boolean
  listViewAnchorRef?: React.RefObject<HTMLDivElement>

}

export function TaskButton({ task, onDragAction,isListView,listViewAnchorRef  }: TaskButtonProps) {
  // Detectar se é uma tarefa com sessão Pomodoro ativa
  const taskWithSession = task as TaskWithActiveSession
  const activeSession = taskWithSession.active_session

  const [isLoading, setIsLoading] = useState(false)
  useEffect(() => {
    setIsLoading(true)
    if (isLoading) {
      console.log(task)
    }
    setIsLoading(false)
  }, [isLoading])

  // Estados baseados no status atual e sessão Pomodoro
  const [isSwapped, setIsSwapped] = useState(
    task.status === "in_progress" || task.status === "waiting"
  )
  const [wasActivated] = useState(false)
  const [isPaused, setIsPaused] = useState(task.status === "paused")
  const [currentTimeRemaining, setCurrentTimeRemaining] = useState(() => {
    const initialTime = calculateTimeRemaining(task)
    // Se a tarefa está em andamento e o tempo é 00:00:00, começar negativo
    if (
      (task.status === "in_progress" || task.status === "waiting") &&
      initialTime.hours === 0 &&
      initialTime.minutes === 0 &&
      initialTime.seconds === 0
    ) {
      return { ...initialTime, isNegative: true }
    }
    return initialTime
  })

  // Estados para drag customizado
  const [isDragging, setIsDragging] = useState(false)
  const [dragStartX, setDragStartX] = useState(0)
  const [currentDragX, setCurrentDragX] = useState(0)
  const [shouldSwapElements, setShouldSwapElements] = useState(false)
  const [lastActionExecuted, setLastActionExecuted] = useState<"start" | "pause" | null>(null)
  const dragRef = useRef<HTMLDivElement>(null)

  // Estados e refs para o modal de edição
  const [isEditModalOpen, setIsEditModalOpen] = useState(false)
  const settingsButtonRef = useRef<HTMLButtonElement>(null)

  const { getTaskRemainingTime } = useTaskStore()

  const timeRemaining = currentTimeRemaining

  const truncateText = (text: string, limit: number) => {
    if (text.length <= limit) return text
    return `${text.substring(0, limit)}...`
  }

  // Buscar tempo real da tarefa quando componente for montado ou task mudar
  useEffect(() => {
    if (task.id) {
      const updateRealTime = async () => {
        try {
          const remainingSeconds = await getTaskRemainingTime(task.id!)
          const duration = secondsToDuration(remainingSeconds)
          setCurrentTimeRemaining(duration)
        } catch (error) {
          // Erro silencioso
        }
      }
      updateRealTime()
    }
  }, [task.id, task.status])

  // Sincronizar estados locais com mudanças do backend
  useEffect(() => {
    const shouldSwap = task.status === "in_progress" || task.status === "waiting"
    const shouldPause = task.status === "paused"

    setIsSwapped(shouldSwap)
    setIsPaused(shouldPause)
  }, [task.status])

  useEffect(() => {
    if (isSwapped && wasActivated && task.id) {
      const startTask = async () => {
        try {
          await invoke("start_task", {
            taskId: task.id,
            stopAndStart: true
          })
        } catch (error) {
          // Erro silencioso
        }
      }

      startTask()
    }
  }, [isSwapped, wasActivated, task.id, task.name])

  // Timer inteligente - Pomodoro ou tempo total
  useEffect(() => {
    let timer: NodeJS.Timeout | null = null

    // Só roda timer se tarefa está ativa e não pausada
    const shouldRunTimer = (task.status === "in_progress" || task.status === "waiting") && !isPaused

    if (shouldRunTimer) {
      // SEMPRE usar o tempo real da tarefa, não do Pomodoro
      // O Pomodoro controla os ciclos, mas o timer deve mostrar o tempo restante da tarefa
      // Permitir timer rodar mesmo quando negativo
      const shouldStartTimer = true // Timer sempre roda quando tarefa está ativa

      if (shouldStartTimer) {
        timer = setInterval(() => {
          setCurrentTimeRemaining(prev => {
            // Calcular total de segundos atuais (considerando se é negativo)
            let totalSeconds = prev.hours * 3600 + prev.minutes * 60 + prev.seconds

            if (prev.isNegative) {
              totalSeconds = -totalSeconds
            }

            // Decrementar 1 segundo
            totalSeconds -= 1

            // Determinar se é negativo
            const isNegative = totalSeconds < 0
            const absSeconds = Math.abs(totalSeconds)

            // Converter de volta para horas, minutos, segundos
            const hours = Math.floor(absSeconds / 3600)
            const minutes = Math.floor((absSeconds % 3600) / 60)
            const seconds = Math.floor(absSeconds % 60)

            // Timer funcionando corretamente

            return { hours, minutes, seconds, isNegative }
          })
        }, 1000)
      }
    }

    return () => {
      if (timer) {
        clearInterval(timer)
      }
    }
  }, [task.status, isPaused, task.id])

  // Atualizar tempo quando tarefa muda ou quando não está ativa
  useEffect(() => {
    if (task.status === "pending" || task.status === "completed") {
      const calculatedTime = calculateTimeRemaining(task)
      setCurrentTimeRemaining(calculatedTime)
    }
  }, [task.status, task.estimated_hours, task])

  // Atualizar tempo sempre que a tarefa for pausada ou retomada
  useEffect(() => {
    if (task.id && task.status !== "pending" && task.status !== "completed") {
      const updateTimeOnStatusChange = async () => {
        try {
          const remainingSeconds = await getTaskRemainingTime(task.id!)
          const duration = secondsToDuration(remainingSeconds)
          setCurrentTimeRemaining(duration)
        } catch (error) {
          // Erro silencioso
        }
      }

      // Adicionar um pequeno delay para garantir que o backend processou a mudança
      setTimeout(updateTimeOnStatusChange, 100)
    }
  }, [task.status, task.id])

  // Handlers para drag customizado com mouse
  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(true)
    setDragStartX(e.clientX)
    setCurrentDragX(e.clientX)
    // NÃO resetar lastActionExecuted aqui
    
    const handleMouseMove = (moveEvent: MouseEvent) => {
      setCurrentDragX(moveEvent.clientX)
      const currentDelta = moveEvent.clientX - dragStartX
    
      // Apenas feedback visual durante o movimento
      if (Math.abs(currentDelta) > 30) {
        if (currentDelta > 0) {
          // DIREITA = PAUSE (executar imediatamente)
          if (
            lastActionExecuted !== "pause" &&
            (task.status === "in_progress" || task.status === "waiting")
          ) {
            const pauseTaskAndReload = async () => {
              try {
                await invoke("pause_task", { taskId: task.id })
                console.log("✅ Tarefa pausada, recarregando dados...")
                if (onDragAction && task.id) {
                  onDragAction(task.id, "pause")
                }
              } catch (error) {
                console.error("❌ Erro ao pausar tarefa:", error)
              }
            }
            pauseTaskAndReload()
            setLastActionExecuted("pause")
          }
          setShouldSwapElements(false)
        } else {
          // ESQUERDA = apenas feedback visual (executar só quando soltar)
          setShouldSwapElements(true)
        }
      } else {
        setShouldSwapElements(false)
      }
    }
  
    const handleMouseUp = (upEvent: MouseEvent) => {
      setIsDragging(false)
      
      const deltaX = upEvent.clientX - dragStartX
  
      // Executar ação de START quando soltar (movimento para esquerda)
      if (Math.abs(deltaX) > 30 && deltaX < 0) {
        // ESQUERDA = START (executar quando soltar)
        console.log("gabriel aqui start task")
        if (
          lastActionExecuted !== "start" &&
          (task.status === "pending" || task.status === "paused")
        ) {
          // Chamar start_task diretamente e depois recarregar dados
          const startTaskAndReload = async () => {
            try {
              await invoke("start_task", {
                taskId: task.id,
                stopAndStart: true
              })
              console.log("✅ Tarefa iniciada, recarregando dados...")
              // Chamar o callback para recarregar dados
              if (onDragAction && task.id) {
                onDragAction(task.id, "start")
              }
            } catch (error) {
              console.error("❌ Erro ao iniciar tarefa:", error)
            }
          }
          startTaskAndReload()
          setLastActionExecuted("start")
        }
      }
  
      // Reset visual elements se não houve movimento suficiente
      if (Math.abs(deltaX) < 30) {
        setShouldSwapElements(false)
      }
  
      // Cleanup
      setCurrentDragX(0)
      // Reset lastActionExecuted após um delay para permitir nova ação
      setTimeout(() => setLastActionExecuted(null), 500)
      window.removeEventListener("mousemove", handleMouseMove)
      window.removeEventListener("mouseup", handleMouseUp)
    }
  
    window.addEventListener("mousemove", handleMouseMove)
    window.addEventListener("mouseup", handleMouseUp)
  }

  // Handlers para drag customizado com touch
  const handleTouchStart = (e: React.TouchEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const touch = e.touches[0]
    setIsDragging(true)
    setDragStartX(touch.clientX)
    setCurrentDragX(touch.clientX)
    // NÃO resetar lastActionExecuted aqui
  
    const handleTouchMove = (moveEvent: TouchEvent) => {
      const touch = moveEvent.touches[0]
      if (touch) {
        setCurrentDragX(touch.clientX)
        const currentDelta = touch.clientX - dragStartX
  
        // Apenas feedback visual durante o movimento
        if (Math.abs(currentDelta) > 30) {
          if (currentDelta > 0) {
            if (
              lastActionExecuted !== "pause" &&
              (task.status === "in_progress" || task.status === "waiting")
            ) {
              const pauseTaskAndReload = async () => {
                try {
                  await invoke("pause_task", { taskId: task.id })
                  console.log("✅ Tarefa pausada, recarregando dados...")
                  if (onDragAction && task.id) {
                    onDragAction(task.id, "pause")
                  }
                } catch (error) {
                  console.error("❌ Erro ao pausar tarefa:", error)
                }
              }
              pauseTaskAndReload()
              setLastActionExecuted("pause")
            }
            setShouldSwapElements(false)
          } else {
            // ESQUERDA = apenas feedback visual (executar só quando soltar)
            setShouldSwapElements(true)
          }
        } else {
          setShouldSwapElements(false)
        }
      }
    }
  
    const handleTouchEnd = (endEvent: TouchEvent) => {
      setIsDragging(false)
  
      const touch = endEvent.changedTouches[0]
      if (touch) {
        const deltaX = touch.clientX - dragStartX
  
        // Executar ação de START quando soltar (movimento para esquerda)
        if (Math.abs(deltaX) > 30 && deltaX < 0) {
          // ESQUERDA = START (executar quando soltar)
          console.log("gabriel aqui start task")
          if (
            lastActionExecuted !== "start" &&
            (task.status === "pending" || task.status === "paused")
          ) {
            // Chamar start_task diretamente e depois recarregar dados
            const startTaskAndReload = async () => {
              try {
                await invoke("start_task", {
                  taskId: task.id,
                  stopAndStart: true
                })
                console.log("✅ Tarefa iniciada, recarregando dados...")
                // Chamar o callback para recarregar dados
                if (onDragAction && task.id) {
                  onDragAction(task.id, "start")
                }
              } catch (error) {
                console.error("❌ Erro ao iniciar tarefa:", error)
              }
            }
            startTaskAndReload()
            setLastActionExecuted("start")
          }
        }
  
        // Reset visual elements se não houve movimento suficiente
        if (Math.abs(deltaX) < 30) {
          setShouldSwapElements(false)
        }
      }
  
      // Cleanup
      setCurrentDragX(0)
      // Reset lastActionExecuted após um delay para permitir nova ação
      setTimeout(() => setLastActionExecuted(null), 500)
      window.removeEventListener("touchmove", handleTouchMove)
      window.removeEventListener("touchend", handleTouchEnd)
    }
  
    window.addEventListener("touchmove", handleTouchMove, { passive: false })
    window.addEventListener("touchend", handleTouchEnd)
  }

  const getButtonStyle = () => {
    // Verificar se a tarefa está em andamento (in_progress ou waiting)
    const isTaskActive = task.status === "in_progress" || task.status === "waiting"

    if (isTaskActive || isSwapped) {
      // Diferenciar visualmente work vs break
      if (task.status === "waiting") {
        return "bg-yellow-500 animate-pulse" // Amarelo para pausas Pomodoro
      }
      // Se estiver atrasado, mostra em vermelho
      if (timeRemaining.isNegative) {
        return "bg-[#FF396D] animate-pulse" // Vermelho para atrasado
      }
      return "bg-[#17FF8B] animate-pulse" // Verde para trabalho em dia
    }
    if (task.status === "paused" || isPaused) {
      return "bg-white animate-pulse" // Branco para pausado manualmente
    }
    return "bg-[#7F7F7F] hover:bg-[#17FF8B] hover:animate-pulse"
  }

  const dragOffset = isDragging ? currentDragX - dragStartX : 0

  const handleSettingsClick = async (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    await invoke("expand_window_for_modal")
    setIsEditModalOpen(true)
  }

  return (
    <>
      <div
        className={`flex select-none items-stretch w-56 gap-2 h-8 bg-[#444444] rounded-full text-white hover:bg-[#252525] transition-all`}
      >
        <button
          ref={settingsButtonRef}
          className="w-[15%] cursor-pointer flex items-center justify-center hover:bg-zinc-600 rounded-l-full transition-colors"

          onClick={handleSettingsClick}
        >
          <SettingsIcon className="w-4 h-4 text-white" />
        </button>

        <div
          className={`flex w-[85%] items-stretch transition-all duration-300 ${
            isSwapped || shouldSwapElements ? "flex-row-reverse" : "flex-row"
          }`}
        >
          <div className="flex flex-col justify-center w-[47%]">
            <span
              className={`font-medium text-xs truncate max-w-full ${
                isSwapped || shouldSwapElements ? "text-center" : ""
              }`}
              title={task.name}
            >
              {truncateText(task.name, 16)}
            </span>
          </div>

          {/* Botão draggable customizado */}
          <div
            ref={dragRef}
            className={`relative w-[53%] flex items-center rounded-full ${
              isDragging
                ? "cursor-grabbing scale-110 shadow-lg z-50"
                : "cursor-grab hover:ring-1 hover:ring-gray-400 hover:scale-105"
            } transition-all duration-200`}
            style={{
              transform: isDragging
                ? `translateX(${Math.min(Math.max(dragOffset, -50), 50)}px)`
                : "translateX(0px)",
              touchAction: "none",
              userSelect: "none"
            }}
            onMouseDown={handleMouseDown}
            onTouchStart={handleTouchStart}
          >
            {/* Background com animação */}
            <div
              className={`absolute inset-0 rounded-full transition-all duration-300 ${getButtonStyle()}`}
            />

            {/* Texto sempre visível */}
            <div
              className={`relative w-full flex flex-col items-center justify-center px-4 text-xs ${
                task.status === "in_progress" || task.status === "waiting" || isSwapped
                  ? "text-black"
                  : ""
              }`}
            >
              {(task.status === "in_progress" || task.status === "waiting") && (
                <span className="font-mono font-bold">
                  {task.status === "waiting"
                    ? activeSession?.session_type === "break"
                      ? "PAUSA"
                      : "BREAK"
                    : timeRemaining.isNegative ? "ATRASADO" : "ANDAMENTO"}
                </span>
              )}
              <span
                className={`font-mono font-bold ${
                  task.status === "in_progress" || task.status === "waiting" ? "text-[10px]" : ""
                } ${task.status === "paused" ? "text-[#FF396D]" : ""}`}
              >
                {task.status === "paused" ? "PAUSADO" : formatTimeDisplay(timeRemaining)}
              </span>
            </div>
          </div>
        </div>
      </div>

      <TaskEditModal
        listViewAnchorRef={listViewAnchorRef}
        isOpen={isEditModalOpen}
        onClose={async () => {
          setIsEditModalOpen(false)
          if(!isListView && !listViewAnchorRef){
            await invoke("reset_window_size")
          }
        }}
        anchorEl={settingsButtonRef}
        task={task}
        displayedWorkedTime={formatTimeDisplay(timeRemaining)}
      />
    </>
  )
}
