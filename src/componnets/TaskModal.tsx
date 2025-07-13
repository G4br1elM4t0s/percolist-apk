import type { RefObject } from "react"
import { useState, useEffect, useMemo } from "react"
import { useTaskStore } from "../store/task.store"
import type { Task } from "../store/task.store"
import { invoke } from "@tauri-apps/api/core"
import { Check, X } from "lucide-react"
import { TimeInput } from "./TimeInput"
import DateInput from "./DateInput"
import { CustomCheckbox } from '../components/CustomCheckbox'

interface TaskModalProps {
  isOpen: boolean
  onClose: () => void
  anchorEl: RefObject<HTMLButtonElement | null>
}

export function TaskModal({ isOpen, onClose, anchorEl }: TaskModalProps) {
  const { addTask } = useTaskStore()
  const [taskName, setTaskName] = useState("")
  const [description, setDescription] = useState("")
  const [timeInput, setTimeInput] = useState("01:00:00")
  const [startDate, setStartDate] = useState(new Date())
  const [endDate, setEndDate] = useState(new Date())
  const [indefiniteEnd, setIndefiniteEnd] = useState(false)
  const [position, setPosition] = useState({ top: 0, left: 0 })
  const [shouldCount, setShouldCount] = useState(true)

  useEffect(() => {
    if (isOpen && anchorEl.current) {
      const rect = anchorEl.current.getBoundingClientRect()
      const modalWidth = 384
      const buttonCenter = rect.left + rect.width / 2
      setPosition({
        top: rect.bottom + 8,
        left: buttonCenter - modalWidth / 2
      })
    }
  }, [isOpen, anchorEl])

  const handleTimeChange = (value: string) => {
    setTimeInput(value)
  }

  const handleAddTask = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()

    if (!taskName.trim()) return

    try {
      const totalHours =
        parseInt(timeInput.split(":")[0]) +
        parseInt(timeInput.split(":")[1]) / 60 +
        parseInt(timeInput.split(":")[2]) / 3600

      const task: Task = {
        name: taskName.trim(),
        description: description.trim() || undefined,
        user: "Gabriel",
        estimated_hours: totalHours,
        worked_hours: 0,
        scheduled_date: startDate.toISOString().split("T")[0],
        end_date: indefiniteEnd ? null : endDate.toISOString().split("T")[0],
        status: "pending" as const,
        created_at: new Date().toISOString(),
        started_at: null,
        completed_at: null,
        should_count: shouldCount,
        count_value: 0
      }

      await addTask(task)
      setTaskName("")
      setDescription("")
      setTimeInput("00:00:00")
      setStartDate(new Date())
      setEndDate(new Date())
      setIndefiniteEnd(false)
      onClose()

      setTimeout(async () => {
        await invoke("reset_window_size")
      }, 100)
    } catch (error: unknown) {
      console.error("Erro detalhado ao adicionar tarefa:", error)
      if (error instanceof Error) {
        alert(`Erro ao adicionar tarefa: ${error.message}`)
      } else {
        alert(`Erro ao adicionar tarefa: ${String(error)}`)
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

  if (!isOpen) return null

  return (
    <div
      className="fixed z-50 bg-zinc-800/90 backdrop-blur-sm rounded-lg shadow-lg w-96 p-6 task-modal"
      style={{
        top: position.top,
        left: position.left
      }}
      onClick={e => e.stopPropagation()}
    >
      <div
        onClick={e => e.stopPropagation()}
        className="w-full max-w-lg  rounded-lg p-6 relative"
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

        <h2 className="text-xl font-semibold text-white text-center mb-6">NOVA TAREFA</h2>

        <form
          onSubmit={e => {
            e.stopPropagation()
            handleAddTask(e)
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
            <div className="flex flex-col  w-3/6 gap-2">
              <label className="text-white/70 text-sm">Tempo estimado</label>
              <TimeInput value={timeInput} onChange={handleTimeChange} />
            </div>
            </div>

            <div className="flex items-center h-16 gap-2 justify-between w-full">
              {/* Campo Data final */}
              <div className="flex flex-col gap-2 w-3/6">
              {!indefiniteEnd && <><label className="text-white/70 text-sm">Data final</label>
                <DateInput value={endDate} onChange={setEndDate} /> </>}
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

            <label className="flex items-center gap-2 cursor-pointer">
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

            {/* Tempo Total */}
            {!indefiniteEnd && calculateTotalTime && (
              <div className="flex items-center justify-center mt-4">
                <span className="text-[#F2F2F2] text-sm font-medium">
                  Tempo Total: {calculateTotalTime}
                </span>
              </div>
            )}

          </div>

          {/* Action Buttons */}
          <div className="flex items-center gap-4 mt-8">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 h-6 bg-[#7F7F7F] hover:bg-white/10 text-white rounded-full  text-xs font-bold  transition-color cursor-pointer"
            >
              CANCELAR
            </button>
            <button
              type="submit"
              className="flex-1 px-4 h-6 bg-[#17FF8B] hover:bg-[#17FF8B]/90 text-xs font-bold  text-black rounded-full transition-colors cursor-pointer"
            >
              CONFIRMAR
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
