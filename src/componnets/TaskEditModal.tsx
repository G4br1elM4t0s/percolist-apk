import type { RefObject } from "react"
import { useState, useEffect, useMemo, useRef } from "react"
import { useTaskStore } from "../store/task.store"
import type { Task } from "../store/task.store"
import { invoke } from "@tauri-apps/api/core"
import { Check, X, Square, Trash2 } from "lucide-react"
import { CustomCheckbox } from '../components/CustomCheckbox'
import DateInput from "./DateInput"
import { TimeInput } from "./TimeInput"

// Controle global melhorado para diferenciar origem dos modais
let currentOpenModalId: string | null = null
let currentCloseFunction: (() => void) | null = null
let currentModalIsFromListView: boolean = false

interface TaskEditModalProps {
  isOpen: boolean
  onClose: () => void
  anchorEl: RefObject<HTMLButtonElement | null>
  task: Task | null
  displayedWorkedTime?: string
  listViewAnchorRef?: React.RefObject<HTMLDivElement>
}

export function TaskEditModal({ isOpen, onClose, anchorEl, task, listViewAnchorRef }: TaskEditModalProps) {
  const { updateTask, completeTask, deleteTask, getTaskTotalWorkedTime } = useTaskStore()
  const [taskName, setTaskName] = useState("")
  const [description, setDescription] = useState("")
  const [timeInput, setTimeInput] = useState("00:00:00")
  const [startDate, setStartDate] = useState(new Date())
  const [endDate, setEndDate] = useState(new Date())
  const [indefiniteEnd, setIndefiniteEnd] = useState(false)
  const [position, setPosition] = useState({ top: 0, left: 0 })
  const [shouldCount, setShouldCount] = useState(false)
  const [isTimeEditable, setIsTimeEditable] = useState(false)
  const [workedTime, setWorkedTime] = useState<number>(0)
  const modalRef = useRef<HTMLDivElement>(null)

  // ID único para este modal baseado na tarefa
  const modalId = task?.id ? `modal-${task.id}` : null
  // Determina se este modal é do ListView
  const isFromListView = !!listViewAnchorRef

  // Carregar o tempo trabalhado quando o modal abrir
  useEffect(() => {
    if (isOpen && task) {
      const fetchWorkedTime = async () => {
        try {
          const totalSeconds = await getTaskTotalWorkedTime(task.id!)
          setWorkedTime(totalSeconds)
        } catch (error) {
          console.error("Erro ao carregar tempo trabalhado:", error)
        }
      }
      fetchWorkedTime()
    }
  }, [isOpen, task, getTaskTotalWorkedTime])

  // Sistema de troca automática de modais com controle de origem
  useEffect(() => {
    if (isOpen && modalId) {
      // Se há outro modal aberto
      if (currentOpenModalId && currentOpenModalId !== modalId && currentCloseFunction) {
        // Regras de fechamento:
        // 1. Se o modal atual é do ListView e o anterior não é: fecha o anterior
        // 2. Se ambos são do mesmo tipo (ListView ou não): fecha o anterior
        // 3. Se o modal atual não é do ListView e o anterior é: não fecha (mantém ListView)

        const shouldCloseCurrentModal =
          isFromListView || // Modal do ListView sempre fecha outros
          (!isFromListView && !currentModalIsFromListView) // Ambos fora do ListView

        if (shouldCloseCurrentModal) {
          currentCloseFunction()
          // Pequeno delay para transição suave
          setTimeout(() => {
            currentOpenModalId = modalId
            currentCloseFunction = onClose
            currentModalIsFromListView = isFromListView
          }, 50)
        } else {
          // Não abre este modal se não deve fechar o anterior
          onClose()
          return
        }
      } else {
        // Registra este modal como aberto
        currentOpenModalId = modalId
        currentCloseFunction = onClose
        currentModalIsFromListView = isFromListView
      }
    } else if (!isOpen && modalId === currentOpenModalId) {
      // Limpa o registro quando este modal fechar
      currentOpenModalId = null
      currentCloseFunction = null
      currentModalIsFromListView = false
    }

    return () => {
      // Cleanup apenas se este modal estava registrado
      if (modalId === currentOpenModalId) {
        currentOpenModalId = null
        currentCloseFunction = null
        currentModalIsFromListView = false
      }
    }
  }, [isOpen, modalId, onClose, isFromListView])

  // Atualizar tempo trabalhado quando o modal abrir e a cada segundo
  useEffect(() => {
    if (isOpen && task?.id && task.started_at && !task.completed_at) {
      // Carregar tempo inicial
      const taskId = task.id
      const fetchWorkedTime = async () => {
        try {
          const totalSeconds = await getTaskTotalWorkedTime(taskId)
          setWorkedTime(totalSeconds)
        } catch (error) {
          console.error("Erro ao carregar tempo trabalhado:", error)
        }
      }

      // Primeira chamada
      fetchWorkedTime()

      // Atualizar a cada segundo
      const interval = setInterval(fetchWorkedTime, 1000)

      // Limpar intervalo quando o modal fechar
      return () => clearInterval(interval)
    }
  }, [isOpen, task?.id, task?.started_at, task?.completed_at, getTaskTotalWorkedTime])

  // Preencher campos quando a tarefa for carregada
  useEffect(() => {
    if (task) {
      setTaskName(task.name)
      setDescription(task.description || "")
      setStartDate(new Date(task.scheduled_date))
      setEndDate(task.end_date ? new Date(task.end_date) : new Date())
      setIndefiniteEnd(!task.end_date)
      setShouldCount(task.should_count)
    }
  }, [JSON.stringify(task)])

  // Atualizar o tempo quando o estado de edição mudar
  useEffect(() => {
    if (task && isTimeEditable) {
      const totalSeconds = Math.round(task.estimated_hours * 3600)
      const hours = Math.floor(totalSeconds / 3600)
      const minutes = Math.floor((totalSeconds % 3600) / 60)
      const seconds = totalSeconds % 60
      setTimeInput(`${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`)
    }
  }, [isTimeEditable, task])

  // Reset isTimeEditable quando o modal fechar
  useEffect(() => {
    if (!isOpen) {
      setIsTimeEditable(false)
    }
  }, [isOpen])

  useEffect(() => {
    if (isOpen && anchorEl.current) {
      const rect = anchorEl.current.getBoundingClientRect()
      setPosition({
        top: listViewAnchorRef ? listViewAnchorRef.current.getBoundingClientRect().top :rect.bottom + 18,
        left:listViewAnchorRef ? listViewAnchorRef.current.getBoundingClientRect().left - 400 : rect.left
      })
    }
  }, [isOpen, anchorEl])

  const handleTimeChange = (value: string) => {
    if (isTimeEditable) {
      setTimeInput(value)
    }
  }

  // Função de validação
  const validateForm = (): { isValid: boolean; errors: string[] } => {
    const errors: string[] = []

    // Validar nome da tarefa
    if (!taskName.trim()) {
      errors.push("Nome da tarefa é obrigatório")
    }

    // Validar tempo estimado se estiver editável
    if (isTimeEditable) {
      const [hours, minutes, seconds] = timeInput.split(":").map(Number)
      if (isNaN(hours) || isNaN(minutes) || isNaN(seconds)) {
        errors.push("Formato de tempo inválido")
      }
      if (hours < 0 || minutes < 0 || seconds < 0 || minutes >= 60 || seconds >= 60) {
        errors.push("Tempo estimado inválido")
      }
    }

    // Validar datas se não for indefinido
    if (!indefiniteEnd) {
      // Normalizar as datas para comparação (zerar horas, minutos, segundos)
      const startDateNormalized = new Date(startDate.getFullYear(), startDate.getMonth(), startDate.getDate())
      const endDateNormalized = new Date(endDate.getFullYear(), endDate.getMonth(), endDate.getDate())

      if (endDateNormalized < startDateNormalized) {
        errors.push("A data final não pode ser anterior à data inicial")
      }
    }

    return {
      isValid: errors.length === 0,
      errors
    }
  }

  const handleUpdateTask = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()

    // Validar formulário antes de submeter
    const validation = validateForm()
    if (!validation.isValid) {
      alert(`Erro de validação:\n${validation.errors.join('\n')}`)
      return
    }

    if (!taskName.trim() || !task) return
    const estimated_hours = isTimeEditable ? (() => {
  const [hours, minutes, seconds] = timeInput.split(":").map(Number)
  const totalSeconds = hours * 3600 + minutes * 60 + seconds
  const totalHours = totalSeconds / 3600

  if (totalHours > 99) {
    return alert("Tempo estimado não pode exceder 99 horas")
  }

  return totalHours
})() : task.estimated_hours



    try {
      const updatedTask: Task = {
        ...task,
        name: taskName.trim(),
        description: description.trim() || undefined,
        // Só envia o novo tempo se estiver desbloqueado
        estimated_hours: isTimeEditable ? (() => {
          const [hours, minutes, seconds] = timeInput.split(":").map(Number)
          const totalSeconds = hours * 3600 + minutes * 60 + seconds
          const totalHours = totalSeconds / 3600

          if (totalHours > 99) {
            alert("Tempo estimado não pode exceder 99 horas")

            throw new Error("Tempo estimado não pode exceder 99 horas")
          }

          return totalHours
        })() : task.estimated_hours,
        worked_hours: workedTime / 3600, // Usando o tempo trabalhado real
        scheduled_date: startDate.toISOString().split("T")[0],
        end_date: indefiniteEnd ? null : endDate.toISOString().split("T")[0],
        should_count: shouldCount
      }

      await updateTask(task.id!, updatedTask)
      onClose()

      setTimeout(async () => {
        await invoke("reset_window_size")
      }, 100)
    } catch (error: unknown) {
      console.error("Erro detalhado ao atualizar tarefa:", error)
      if (error instanceof Error) {
        alert(`Erro ao atualizar tarefa: ${error.message}`)
      } else {
        alert(`Erro ao atualizar tarefa: ${String(error)}`)
      }
    }
  }

  const calculateTotalTime = useMemo(() => {
    if (indefiniteEnd) return null;

    try {
      // Normalizar as datas para comparação (zerar horas, minutos, segundos)
      const start = new Date(startDate.getFullYear(), startDate.getMonth(), startDate.getDate());
      const end = new Date(endDate.getFullYear(), endDate.getMonth(), endDate.getDate());

      // Calcular diferença em dias (sempre >= 0)
      const diffInDays = Math.max(0, Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24)));

      // Converte o timeInput (HH:mm:ss) em segundos
      const [hours, minutes, seconds] = timeInput.split(':').map(Number);
      const totalSeconds = (hours * 3600) + (minutes * 60) + seconds;

      // Adiciona os dias convertidos em segundos
      const totalSecondsWithDays = totalSeconds + (diffInDays * 24 * 3600);

      // Converte para o formato desejado
      const d = Math.floor(totalSecondsWithDays / (3600 * 24));
      const h = Math.floor((totalSecondsWithDays % (3600 * 24)) / 3600);
      const m = Math.floor((totalSecondsWithDays % 3600) / 60);
      const s = totalSecondsWithDays % 60;

      const result = `${String(d).padStart(2, '0')}d${String(h).padStart(2, '0')}h${String(m).padStart(2, '0')}m${String(s).padStart(2, '0')}s`;

      return result;
    } catch (error) {
      console.error('❌ Erro ao calcular tempo total:', error);
      return null;
    }
  }, [startDate, endDate, indefiniteEnd, timeInput]);

  // Fechar modal ao clicar fora
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (modalRef.current && !modalRef.current.contains(event.target as Node)) {
        onClose()
      }
    }

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside)
    } else {
      document.removeEventListener("mousedown", handleClickOutside)
    }

    return () => {
      document.removeEventListener("mousedown", handleClickOutside)
    }
  }, [isOpen, onClose])

  if (!isOpen || !task) return null

  return (
    <div
      ref={modalRef}
      className="fixed select-none z-50 bg-zinc-800/90 backdrop-blur-sm rounded-lg shadow-lg w-96 p-6 task-modal"
      style={{
        top: position.top,
        left: position.left
      }}
      onClick={e => e.stopPropagation()}
    >
      <div
        onClick={e => e.stopPropagation()}
        className="w-full max-w-lg rounded-lg p-6 relative"
      >
        <button
          onClick={e => {
            e.stopPropagation()
            onClose()
          }}
          className="absolute right-6 top-1 text-white/70 hover:text-white transition-colors"
        >
          <X className="w-5 h-5" />
        </button>

        <h2 className="text-xl font-semibold text-white text-center mb-6">EDITAR TAREFA</h2>

        <form
          onSubmit={e => {
            e.stopPropagation()
            handleUpdateTask(e)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
            }
          }}
          className="flex flex-col gap-4"
          onClick={e => e.stopPropagation()}
        >
          {/* Task Name */}
          <div className="flex flex-col gap-2">
            <input
              type="text"
              value={taskName}
              onChange={e => setTaskName(e.target.value)}
              placeholder="Nome da Tarefa"
              className="w-full px-4 py-2 bg-[#D9D9D9] rounded-lg text-[#181818] placeholder:text-[#181818] placeholder:font-medium placeholder:pl-2 focus:outline-none text-base"
              autoFocus
              required
            />
          </div>

          {/* Description */}
          <div className="flex flex-col gap-2">
            <textarea
              value={description}
              onChange={e => setDescription(e.target.value)}
              placeholder="Descrição"
              className="w-full px-4 py-3 bg-[#D9D9D9] rounded-lg text-[#181818] placeholder:font-medium placeholder:text-[#181818] focus:outline-none min-h-[100px] resize-none text-base"
            />
          </div>

          {/* Date Inputs */}
          <div className="flex flex-col gap-4">
            <div className="flex items-center justify-between">
              <div className="flex flex-col gap-2 w-3/6">
                <label className="text-white/70 text-sm">Data inicial</label>
                <DateInput value={startDate} onChange={setStartDate} />
              </div>
              <div className="flex flex-col w-3/6 gap-2">
                <div className="flex items-center gap-2">
                  <label className="text-white/70 text-sm">TEMPO ESTIMADO</label>
                  <button
                    type="button"
                    onClick={() => setIsTimeEditable(!isTimeEditable)}
                    className={`p-1 rounded ${isTimeEditable ? 'bg-green-500' : 'bg-gray-500'}`}
                  >
                    {isTimeEditable ? '🔓' : '🔒'}
                  </button>
                </div>
                <TimeInput
                  value={timeInput}
                  onChange={handleTimeChange}
                  disabled={!isTimeEditable}
                  className={!isTimeEditable ? 'opacity-50' : ''}
                />
                <div className="text-sm text-gray-500">
                  Tempo Trabalhado: {formatTime(workedTime)}
                </div>
              </div>
            </div>

            <div className="flex items-center h-16 gap-2 justify-between w-full">
              {/* Campo Data final */}
              <div className="flex flex-col gap-2 w-3/6">
                {!indefiniteEnd && (
                  <>
                    <label className="text-white/70 text-sm">Data final</label>
                    <DateInput value={endDate} onChange={setEndDate} />
                  </>
                )}
              </div>

              {/* Checkbox Contabilizar */}
              <div className="flex items-end h-full pb-2 w-3/6">
                <CustomCheckbox
                  checked={shouldCount}
                  onChange={setShouldCount}
                  label="CONTABILIZAR TAREFA"
                  className="whitespace-nowrap"
                />
              </div>
            </div>

         <div className="w-full ">
  <label className="w-fit flex items-center gap-2 cursor-pointer">
    <input
      type="checkbox"
      checked={indefiniteEnd}
      onChange={e => setIndefiniteEnd(e.target.checked)}
      className="sr-only"
    />
    <div className={`w-3 h-3 border rounded ${indefiniteEnd ? 'bg-[#17FF8B] border-[#17FF8B]' : 'border-white/30'} flex items-center justify-center`}>
      {indefiniteEnd && <Check className="w-2 h-2 text-black" />}
    </div>
    <span className="text-white/70 text-xs">Indefinir data final</span>
  </label>
</div>
            {/* Tempo Total */}
            {!indefiniteEnd && calculateTotalTime && (
              <div className="flex items-center justify-center mt-4">
                <span className="text-[#F2F2F2] select-none text-sm font-medium">
                  Tempo Total: {calculateTotalTime}
                </span>
              </div>
            )}
          </div>

          {/* Quick Action Buttons */}
          <div className="flex gap-2 mt-8">
            {task.status !== "completed" && (
              <button
                type="button"
                onClick={async () => {
                  if (task.id) {
                    await completeTask(task.id)
                    onClose()
                  }
                }}
                className="flex-1 h-8 flex items-center justify-center gap-2 cursor-pointer bg-[#17FF8B]/80 hover:bg-[#17FF8B] text-black rounded-full transition-colors"
              >
                <Square className="w-3 h-3" />
                Concluir
              </button>
            )}

            <button
              type="button"
              onClick={async () => {
                if (task.id && confirm("Tem certeza que deseja excluir esta tarefa?")) {
                  await deleteTask(task.id)
                  onClose()
                }
              }}
              className="h-8 w-8 flex items-center justify-center cursor-pointer bg-red-600/80 hover:bg-red-600 text-white rounded-full transition-colors"
            >
              <Trash2 className="w-3 h-3" />
            </button>
          </div>

          {/* Action Buttons */}
          <div className="flex items-center gap-4 mt-4">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 h-6 bg-[#7F7F7F] hover:bg-white/10 text-white rounded-full text-xs font-bold transition-color cursor-pointer"
            >
              CANCELAR
            </button>
            <button
              type="submit"
              className="flex-1 px-4 h-6 bg-[#17FF8B] hover:bg-[#17FF8B]/90 text-xs font-bold text-black rounded-full transition-colors cursor-pointer"
            >
              ATUALIZAR
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

// Função auxiliar para formatar o tempo
function formatTime(seconds: number): string {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const remainingSeconds = seconds % 60
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(remainingSeconds).padStart(2, '0')}`
}
