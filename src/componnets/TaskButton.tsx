import { invoke } from "@tauri-apps/api/core"
import { useEffect, useState, useRef, useCallback } from "react"
import type { Task, TaskWithActiveSession } from "../store/task.store"
import { formatTimeDisplay } from "../utils/format"
import { calculateTimeRemaining, secondsToDuration } from "../utils/time"
import { SettingsIcon } from "../components/SettingsIcon"
import { useTaskStore } from "../store/task.store"
import { TaskEditModal } from "./TaskEditModal"
import { useQueryClient } from "@tanstack/react-query";

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
  const [currentTimeRemaining, setCurrentTimeRemaining] = useState(() => ({ hours: 0, minutes: 0, seconds: 0, isNegative: false }))

  // Configuração da sincronização periódica
  const SYNC_INTERVAL_SECONDS = 10 // Sincroniza a cada 10 segundos

  // Timer sem drift: saldo base + timestamp da última sync
  const baseRemainingRef = useRef<number>(0)
  const lastSyncAtRef = useRef<number>(Date.now())
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const isUpdatingBackendRef = useRef<boolean>(false) // Controle de atualização do backend

  // Estados para drag customizado melhorado
  const [isDragging, setIsDragging] = useState(false)
  const [dragStartX, setDragStartX] = useState(0)
  const [currentDragX, setCurrentDragX] = useState(0)
  const [shouldSwapElements, setShouldSwapElements] = useState(false)
  const [lastActionExecuted, setLastActionExecuted] = useState<"start" | "pause" | null>(null)
  const [dragProgress, setDragProgress] = useState(0) // Progresso do drag (0-100)
  const [dragDirection, setDragDirection] = useState<"left" | "right" | null>(null)
  const dragRef = useRef<HTMLDivElement>(null)

  // Configurações do drag and drop
  const DRAG_CONFIG = {
    THRESHOLD: 25, // Distância mínima para ativar (px)
    SENSITIVITY: 1.2, // Sensibilidade do movimento
    ANIMATION_DURATION: 200, // Duração das animações (ms)
    HAPTIC_FEEDBACK: true, // Feedback tátil para dispositivos touch
    VISUAL_FEEDBACK: true, // Feedback visual aprimorado
    MAX_DRAG_DISTANCE: 100, // Distância máxima para cálculo de progresso
    ELASTIC_BOUNCE: true, // Efeito elástico ao atingir limites
    SNAP_BACK: true // Retorna suavemente à posição original
  }

  // Configurações para proteção contra drift de timer
  const DRIFT_PROTECTION = {
    INACTIVITY_THRESHOLD: 10, // Segundos de inatividade para forçar sync
    DRIFT_THRESHOLD: 2, // Segundos de drift para corrigir automaticamente
    NORMAL_SYNC_THRESHOLD: 5, // Segundos para sync normal
    SYNC_INTERVAL_SECONDS: 10, // Intervalo para sincronização periódica
    SYNC_DEBUG: true // Logs de debug para sincronização
  }
  const queryClient = useQueryClient()
  // Estados e refs para o modal de edição
  const [isEditModalOpen, setIsEditModalOpen] = useState(false)
  const settingsButtonRef = useRef<HTMLButtonElement>(null)

  const { getTaskRemainingTime } = useTaskStore()

  const timeRemaining = currentTimeRemaining

  // Função para haptic feedback em dispositivos touch
  const triggerHapticFeedback = useCallback((intensity: "light" | "medium" | "heavy" = "medium") => {
    if (!DRAG_CONFIG.HAPTIC_FEEDBACK) return

    try {
      if (navigator.vibrate) {
        const patterns = {
          light: [10],
          medium: [20],
          heavy: [30]
        }
        navigator.vibrate(patterns[intensity])
      }
    } catch (error) {
      // Fallback silencioso se não suportar
    }
  }, [])

  // Função para atualizar o backend periodicamente sem interferir no contador
  const updateBackendPeriodically = useCallback(async (taskId: string, currentRemainingSeconds: number) => {
    // Proteção contra múltiplas chamadas simultâneas
    if (isUpdatingBackendRef.current) {
      if (DRIFT_PROTECTION.SYNC_DEBUG) {
        console.log(`⏳ Sync já em andamento, ignorando...`)
      }
      return
    }

    isUpdatingBackendRef.current = true

    try {
      // Atualiza o backend com o tempo atual
      await invoke("update_task_remaining_time", {
        taskId: taskId,
        remainingSeconds: currentRemainingSeconds
      })

      // Log para debug (opcional)
      if (DRIFT_PROTECTION.SYNC_DEBUG) {
        console.log(`🔄 Backend atualizado: ${currentRemainingSeconds}s para tarefa ${taskId}`)
      }

      // Atualiza o timestamp de sync para manter precisão
      lastSyncAtRef.current = Date.now()
      baseRemainingRef.current = currentRemainingSeconds

      // Log de debug adicional
      if (DRIFT_PROTECTION.SYNC_DEBUG) {
        const now = Date.now()
        const elapsedSinceSync = Math.floor((now - lastSyncAtRef.current) / 1000)
        console.log(`⏱️ Sync realizado: ${elapsedSinceSync}s desde último sync, tempo restante: ${currentRemainingSeconds}s`)
      }

    } catch (error) {
      console.error("❌ Erro ao atualizar backend periodicamente:", error)
    } finally {
      isUpdatingBackendRef.current = false
    }
  }, [])

  // 🔧 SINCRONIZAÇÃO CRÍTICA: Atualiza o backend com o tempo atual antes de pausar
  const syncTimeBeforePause = useCallback(async (taskId: string) => {
    try {
      const currentElapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)
      const currentRemaining = baseRemainingRef.current - currentElapsed

      console.log(`⏸️ Sincronizando tempo antes de pausar: ${currentRemaining}s (${currentRemaining < 0 ? 'negativo' : 'positivo'})`)

      // Atualiza o backend com o tempo atual ANTES de pausar
      await invoke("update_task_remaining_time", {
        taskId,
        remainingSeconds: currentRemaining
      })

      console.log(`✅ Tempo sincronizado com sucesso: ${currentRemaining}s`)
      return currentRemaining
    } catch (error) {
      console.error("❌ Erro ao sincronizar tempo antes de pausar:", error)
      // Retorna o tempo calculado mesmo se falhar o sync
      const currentElapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)
      const currentRemaining = baseRemainingRef.current - currentElapsed
      console.log(`🔄 Retornando tempo calculado localmente: ${currentRemaining}s`)
      return currentRemaining
    }
  }, [])

  const truncateText = (text: string, limit: number) => {
    if (text.length <= limit) return text
    return `${text.substring(0, limit)}...`
  }

  // Sincroniza com backend e atualiza saldo base/timestamp + estado
  const syncFromBackend = useCallback(async (taskId: string, forceSync = false) => {
    try {
      const remainingSeconds = await getTaskRemainingTime(taskId)

      // PROTEÇÃO CRÍTICA: Se a tarefa está pausada, NÃO faz sync para evitar reset de tempo
      // Mas permite sync forçado para operações críticas como pausar
      if (task.status === "paused" && !forceSync) {
        console.log(`🚫 Sync bloqueado: tarefa pausada, mantendo tempo atual`)
        return
      }

      // Permite sync de tempos negativos para permitir pausar e salvar
      // Removido o return early para remainingSeconds < 0

      // Proteção contra re-sync desnecessário durante timer ativo
      if (!forceSync && intervalRef.current) {
        // Se o timer está rodando, só faz sync se a diferença for significativa (>5s)
        const currentElapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)
        const currentCalculated = baseRemainingRef.current - currentElapsed
        const difference = Math.abs(remainingSeconds - currentCalculated)

        if (difference < 5) {
          // Diferença pequena, não precisa re-sync
          console.log(`🔄 Sync ignorado: diferença pequena (${difference}s)`)
          return
        }
        console.log(`🔄 Sync necessário: diferença significativa (${difference}s)`)
      }

      baseRemainingRef.current = Number.isFinite(remainingSeconds) ? remainingSeconds : 0
      lastSyncAtRef.current = Date.now()

      const isNegative = remainingSeconds < 0
      const dur = secondsToDuration(Math.abs(remainingSeconds))
      setCurrentTimeRemaining({ ...dur, isNegative })
    } catch (_) {
      // silencioso
    }
  }, [getTaskRemainingTime, task.status])

  // Atualiza display quando pendente/completa; NÃO chama sync aqui para evitar resets
  useEffect(() => {
    if (!task.id) return
    if (task.status === "pending" || task.status === "completed") {
      const calculatedTime = calculateTimeRemaining(task)
      setCurrentTimeRemaining(calculatedTime)
      if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null }
    }
  }, [task.id, task.status, task.estimated_hours])

  // Sync inicial ao trocar de tarefa (só se não estiver pausada)
  useEffect(() => {
    if (task.id && task.status !== "paused") {
      // Permite sync mesmo para tempos negativos
      syncFromBackend(task.id)
    }
  }, [task.id, task.status, syncFromBackend])

  // Sincroniza estados locais com mudanças do backend
  useEffect(() => {
    const shouldSwap = task.status === "in_progress" || task.status === "waiting"
    const shouldPause = task.status === "paused"

    setIsSwapped(shouldSwap)
    setIsPaused(shouldPause)

    // Quando pausa automaticamente, resetar o drag visual para direita
    if (shouldPause) {
      setShouldSwapElements(false)
      setLastActionExecuted(null) // Reset da ação de drag também
    }
  }, [task.status])

  useEffect(() => {
    if (isSwapped && wasActivated && task.id) {
      const startTask = async () => {
        try {
          await invoke("start_task", {
            taskId: task.id,
            stopAndStart: true
          })
          console.log("deveria invalidar query")
          queryClient.invalidateQueries({ queryKey: ["active-task"] })
        } catch (error) {
          // Erro silencioso
        }
      }

      startTask()
    }
  }, [isSwapped, wasActivated, task.id, task.name])

  // Timer sem drift: apenas calcula contra timestamp + saldo base
  useEffect(() => {
    const isActive = ((task.status === "in_progress" || task.status === "waiting" || !!activeSession) && task.status !== "paused" && !!task.id)

    // limpar intervalo anterior sempre
    if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null }
    if (!isActive) return

    let canceled = false
    let lastSyncTime = 0 // Proteção contra múltiplas syncs
    let syncCounter = 0 // Contador para sincronização periódica

    ;(async () => {
      // Sync inicial ao iniciar o timer (FORÇADO para garantir precisão)
      const now = Date.now()
      if (now - lastSyncTime > 1000) { // Evita múltiplas syncs em 1s
        lastSyncTime = now
        await syncFromBackend(task.id!, true)
      }
      if (canceled) return

      intervalRef.current = setInterval(() => {
        // PROTEÇÃO CRÍTICA: Se a tarefa foi pausada durante a execução, para o timer
        if (task.status === "paused") {
          console.log(`⏸️ Timer pausado durante execução, parando...`)
          clearInterval(intervalRef.current!)
          intervalRef.current = null
          return
        }

        const elapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)
        const rawRemaining = baseRemainingRef.current - elapsed
        const remaining = rawRemaining // pode ser negativo, exibimos sinal

        // Proteção contra drift: se o tempo calculado for muito diferente do esperado
        const expectedElapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)
        const timeDrift = Math.abs(expectedElapsed - elapsed)

        if (timeDrift > DRIFT_PROTECTION.DRIFT_THRESHOLD) { // Se houve drift significativo
          console.log(`⚠️ Drift detectado: ${timeDrift}s, corrigindo...`)
          // Corrige o drift forçando uma sincronização
          syncFromBackend(task.id!, true)
        }

        const dur = secondsToDuration(Math.abs(remaining))
        setCurrentTimeRemaining({ ...dur, isNegative: remaining < 0 })

        // Sincronização periódica com o backend
        syncCounter++
        if (syncCounter >= DRIFT_PROTECTION.SYNC_INTERVAL_SECONDS) {
          syncCounter = 0
          // Atualiza o backend com o tempo atual sem interferir no contador
          updateBackendPeriodically(task.id!, remaining)
        }

        // Continua decrementando mesmo após zero (tempo negativo)
        // Só para quando a tarefa muda de status ou é pausada
      }, 1000)
    })()

    return () => {
      canceled = true
      if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null }
    }
  }, [task.status, task.id, syncFromBackend, activeSession, updateBackendPeriodically])

  // Re-sync ao voltar visível (uma única vez, não a cada tick)
  useEffect(() => {
    const onVis = () => {
      // PROTEÇÃO CRÍTICA: Não faz sync se a tarefa está pausada
      if (document.visibilityState === "visible" && task.id && task.status !== "paused") {
        // Detecta se houve inatividade prolongada (mais de 10 segundos)
        const currentElapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)

        if (currentElapsed > DRIFT_PROTECTION.INACTIVITY_THRESHOLD) {
          // Inatividade prolongada detectada - FORÇA sync para corrigir drift
          console.log(`🔄 Inatividade detectada: ${currentElapsed}s, forçando sync...`)
          syncFromBackend(task.id, true) // forceSync = true
        } else if (currentElapsed > DRIFT_PROTECTION.NORMAL_SYNC_THRESHOLD) {
          // Re-sync normal se passou mais de 5s
          syncFromBackend(task.id)
        }
      }
    }
    document.addEventListener("visibilitychange", onVis)
    return () => document.removeEventListener("visibilitychange", onVis)
  }, [task.id, task.status, syncFromBackend])

  // Atualizar tempo quando tarefa muda ou quando não está ativa
  useEffect(() => {
    if (task.status === "pending" || task.status === "completed") {
      const calculatedTime = calculateTimeRemaining(task)
      setCurrentTimeRemaining(calculatedTime)
    }
  }, [task.status, task.estimated_hours, task])

  // Atualizar tempo em retomada (não faz fetch ao pausar para não "resetar")
  useEffect(() => {
    const isResumed = task.status === "in_progress" || task.status === "waiting"
    // PROTEÇÃO CRÍTICA: Só executa se realmente houve resume (não estava pausada antes)
    if (task.id && isResumed && task.status !== "paused") {
      const updateTimeOnResume = async () => {
        try {
          const remainingSeconds = await getTaskRemainingTime(task.id!)

          // Permite resume mesmo para tempos negativos
          // Removido o return early para remainingSeconds <= 0

          const duration = secondsToDuration(remainingSeconds)
          setCurrentTimeRemaining(duration)
          // ressincroniza base p/ o timer continuar consistente pós-resume
          baseRemainingRef.current = Number.isFinite(remainingSeconds) ? remainingSeconds : 0
          lastSyncAtRef.current = Date.now()

          // Força re-sync para garantir precisão após resume
          setTimeout(() => syncFromBackend(task.id!, true), 50)
        } catch (error) {
          // Erro silencioso
        }
      }
      setTimeout(updateTimeOnResume, 100)
    }
  }, [task.status, task.id, getTaskRemainingTime, syncFromBackend])

  // Monitor de mudanças de estado para re-sync inteligente
  useEffect(() => {
    // PROTEÇÃO CRÍTICA: Não executa se a tarefa está pausada
    if (!task.id || !intervalRef.current || task.status === "paused") return

    // Se o timer está rodando e houve mudança significativa de estado
    const shouldReSync = () => {
      const currentElapsed = Math.floor((Date.now() - lastSyncAtRef.current) / 1000)
      const currentCalculated = baseRemainingRef.current - currentElapsed

      // Re-sync se passou muito tempo desde a última sync (>30s)
      if (currentElapsed > 30) {
        return true
      }

      return false
    }

    if (shouldReSync()) {
      // Re-sync inteligente sem forçar
      syncFromBackend(task.id)
    }
  }, [task.status, task.id, syncFromBackend])

  // Handlers para drag customizado com mouse melhorado
  const handleMouseDown = (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()

    // Reset estados de drag
    setIsDragging(true)
    setDragStartX(e.clientX)
    setCurrentDragX(e.clientX)
    setDragProgress(0)
    setDragDirection(null)

    // NÃO resetar lastActionExecuted aqui

    const handleMouseMove = (moveEvent: MouseEvent) => {
      setCurrentDragX(moveEvent.clientX)
      const currentDelta = moveEvent.clientX - dragStartX
      const absDelta = Math.abs(currentDelta)

              // Calcula progresso do drag (0-100)
        const maxDrag = DRAG_CONFIG.MAX_DRAG_DISTANCE
        const progress = Math.min((absDelta / maxDrag) * 100, 100)
        setDragProgress(progress)

      // Define direção do drag
      if (absDelta > DRAG_CONFIG.THRESHOLD) {
        const newDirection = currentDelta > 0 ? "right" : "left"
        if (dragDirection !== newDirection) {
          setDragDirection(newDirection)
          // Haptic feedback ao mudar direção
          triggerHapticFeedback("light")
        }
      } else {
        setDragDirection(null)
      }

      // Feedback visual aprimorado durante o movimento
      if (absDelta > DRAG_CONFIG.THRESHOLD) {
        if (currentDelta > 0) {
          // DIREITA = PAUSE (executar imediatamente)
          console.log(`🔄 Drag para direita detectado: status=${task.status}, lastAction=${lastActionExecuted}`)

          if (
            lastActionExecuted !== "pause" &&
            (task.status === "in_progress" || task.status === "waiting")
          ) {
            console.log(`✅ Condições para pausar atendidas, executando...`)

            const pauseTaskAndReload = async () => {
              try {
                console.log("⏸️ Iniciando processo de pausar tarefa...")

                // 🔧 SINCRONIZAÇÃO CRÍTICA: Sincroniza tempo antes de pausar
                const syncedTime = await syncTimeBeforePause(task.id!)

                if (syncedTime !== null && syncedTime !== undefined) {
                  console.log(`✅ Tempo sincronizado: ${syncedTime}s, pausando tarefa...`)

                  // Agora pausa a tarefa
                  await invoke("pause_task", { taskId: task.id })
                  console.log("✅ Tarefa pausada com sucesso!")

                  // Recarrega dados se callback disponível
                  if (onDragAction && task.id) {
                    onDragAction(task.id, "pause")
                  }
                } else {
                  console.error("❌ Falha ao sincronizar tempo, tentando pausar mesmo assim...")

                  // Tenta pausar mesmo sem sync bem-sucedido
                  await invoke("pause_task", { taskId: task.id })
                  console.log("✅ Tarefa pausada (sem sync de tempo)")

                  if (onDragAction && task.id) {
                    onDragAction(task.id, "pause")
                  }
                }
              } catch (error) {
                console.error("❌ Erro ao pausar tarefa:", error)
              }
            }
            pauseTaskAndReload()
            setLastActionExecuted("pause")
            // Haptic feedback ao pausar
            triggerHapticFeedback("medium")
          } else {
            console.log(`❌ Condições para pausar não atendidas: lastAction=${lastActionExecuted}, status=${task.status}`)
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
      const absDelta = Math.abs(deltaX)

      // Executar ação de START quando soltar (movimento para esquerda)
      if (absDelta > DRAG_CONFIG.THRESHOLD && deltaX < 0) {
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
              queryClient.invalidateQueries({ queryKey: ["active-task"] })
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
            // Haptic feedback ao iniciar
            triggerHapticFeedback("medium")
          }
        }

      // Reset visual elements se não houve movimento suficiente
      if (absDelta < DRAG_CONFIG.THRESHOLD) {
        setShouldSwapElements(false)
      }

      // Cleanup com animação suave
      setCurrentDragX(0)
      setDragProgress(0)
      setDragDirection(null)

      // Reset lastActionExecuted após um delay para permitir nova ação
      setTimeout(() => setLastActionExecuted(null), 500)
      window.removeEventListener("mousemove", handleMouseMove)
      window.removeEventListener("mouseup", handleMouseUp)
    }

    window.addEventListener("mousemove", handleMouseMove)
    window.addEventListener("mouseup", handleMouseUp)
  }

  // Handlers para drag customizado com touch melhorado
  const handleTouchStart = (e: React.TouchEvent) => {
    e.preventDefault()
    e.stopPropagation()
    const touch = e.touches[0]

    // Reset estados de drag
    setIsDragging(true)
    setDragStartX(touch.clientX)
    setCurrentDragX(touch.clientX)
    setDragProgress(0)
    setDragDirection(null)

    // NÃO resetar lastActionExecuted aqui

    const handleTouchMove = (moveEvent: TouchEvent) => {
      const touch = moveEvent.touches[0]
      if (touch) {
        setCurrentDragX(touch.clientX)
        const currentDelta = touch.clientX - dragStartX
        const absDelta = Math.abs(currentDelta)

        // Calcula progresso do drag (0-100)
        const maxDrag = DRAG_CONFIG.MAX_DRAG_DISTANCE
        const progress = Math.min((absDelta / maxDrag) * 100, 100)
        setDragProgress(progress)

        // Define direção do drag
        if (absDelta > DRAG_CONFIG.THRESHOLD) {
          const newDirection = currentDelta > 0 ? "right" : "left"
          if (dragDirection !== newDirection) {
            setDragDirection(newDirection)
            // Haptic feedback ao mudar direção
            triggerHapticFeedback("light")
          }
        } else {
          setDragDirection(null)
        }

        // Feedback visual aprimorado durante o movimento
        if (absDelta > DRAG_CONFIG.THRESHOLD) {
          if (currentDelta > 0) {
            console.log(`🔄 Drag para direita detectado (touch): status=${task.status}, lastAction=${lastActionExecuted}`)

            if (
              lastActionExecuted !== "pause" &&
              (task.status === "in_progress" || task.status === "waiting")
            ) {
              console.log(`✅ Condições para pausar atendidas (touch), executando...`)

              const pauseTaskAndReload = async () => {
                try {
                  console.log("⏸️ Iniciando processo de pausar tarefa (touch)...")

                  // 🔧 SINCRONIZAÇÃO CRÍTICA: Sincroniza tempo antes de pausar
                  const syncedTime = await syncTimeBeforePause(task.id!)

                  if (syncedTime !== null && syncedTime !== undefined) {
                    console.log(`✅ Tempo sincronizado: ${syncedTime}s, pausando tarefa...`)

                    // Agora pausa a tarefa
                    await invoke("pause_task", { taskId: task.id })
                    console.log("✅ Tarefa pausada com sucesso!")

                    // Recarrega dados se callback disponível
                    if (onDragAction && task.id) {
                      onDragAction(task.id, "pause")
                    }
                  } else {
                    console.error("❌ Falha ao sincronizar tempo, tentando pausar mesmo assim...")

                    // Tenta pausar mesmo sem sync bem-sucedido
                    await invoke("pause_task", { taskId: task.id })
                    console.log("✅ Tarefa pausada (sem sync de tempo)")

                    if (onDragAction && task.id) {
                      onDragAction(task.id, "pause")
                    }
                  }
                } catch (error) {
                  console.error("❌ Erro ao pausar tarefa:", error)
                }
              }
              pauseTaskAndReload()
              setLastActionExecuted("pause")
              // Haptic feedback ao pausar
              triggerHapticFeedback("medium")
            } else {
              console.log(`❌ Condições para pausar não atendidas (touch): lastAction=${lastActionExecuted}, status=${task.status}`)
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
        const absDelta = Math.abs(deltaX)

        // Executar ação de START quando soltar (movimento para esquerda)
        if (absDelta > DRAG_CONFIG.THRESHOLD && deltaX < 0) {
          // ESQUERDA = START (executar quando soltar)
          console.log("gabriel aqui start task")
          if (
            lastActionExecuted !== "start" &&
            (task.status === "pending" || task.status === "paused")
          ) {
            const startTaskAndReload = async () => {
              try {
                await invoke("start_task", {
                  taskId: task.id,
                  stopAndStart: true
                })
                console.log("✅ Tarefa iniciada, recarregando dados...")
                queryClient.invalidateQueries({ queryKey: ["active-task"] })
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
            // Haptic feedback ao iniciar
            triggerHapticFeedback("medium")
          }
        }

        // Reset visual elements se não houve movimento suficiente
        if (absDelta < DRAG_CONFIG.THRESHOLD) {
          setShouldSwapElements(false)
        }
      }

      // Cleanup com animação suave
      setCurrentDragX(0)
      setDragProgress(0)
      setDragDirection(null)

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

  // Calcula estilos de feedback visual para o drag
  const getDragFeedbackStyle = () => {
    if (!isDragging) return {}

    const opacity = Math.min(dragProgress / 100, 0.8)
    const scale = 1 + (dragProgress / 100) * 0.05

    return {
      opacity: 0.8 + opacity * 0.2,
      transform: `scale(${scale})`,
      transition: `all ${DRAG_CONFIG.ANIMATION_DURATION}ms ease-out`
    }
  }

  // Estilo para indicador de direção
  const getDirectionIndicatorStyle = () => {
    if (!dragDirection || !isDragging) return {}

    const baseColor = dragDirection === "left" ? "#17FF8B" : "#FF6B6B"
    const intensity = Math.min(dragProgress / 100, 1)

    return {
      backgroundColor: baseColor,
      opacity: intensity * 0.3,
      transform: `scale(${0.8 + intensity * 0.4})`,
      transition: `all ${DRAG_CONFIG.ANIMATION_DURATION}ms ease-out`
    }
  }

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
            // Se está pausada, sempre fica na direita (não invertido)
            // Se está ativa, pode estar invertido por drag ou status
            task.status === "paused" ? "flex-row" : (isSwapped || shouldSwapElements ? "flex-row-reverse" : "flex-row")
          }`}
        >
          <div className="flex flex-col justify-center w-[47%]">
            <span
              className={`font-medium text-xs truncate max-w-full ${
                // Se está pausada, sempre centraliza na direita
                // Se está ativa, centraliza se estiver invertida
                task.status === "paused" ? "text-center" : (isSwapped || shouldSwapElements ? "text-center" : "")
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
              userSelect: "none",
              ...getDragFeedbackStyle()
            }}
            onMouseDown={handleMouseDown}
            onTouchStart={handleTouchStart}
          >
            {/* Indicador de direção do drag */}
            {isDragging && dragDirection && (
              <div
                className="absolute inset-0 rounded-full pointer-events-none z-10"
                style={getDirectionIndicatorStyle()}
              />
            )}

            {/* Indicador de progresso do drag */}

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
