import { type Task } from "../store/task.store"
import { Filter } from "lucide-react"

interface TaskFilterProps {
  selectedFilters: Set<Task["status"]>
  onToggleFilter: (status: Task["status"]) => void
  onClear: () => void
  onSelectAll: (allStatuses: Task["status"][]) => void

  isOpen: boolean
  onClose: () => void
  embedded?: boolean
}

export function TaskFilter({
  selectedFilters,
  onToggleFilter,
  onClear,
  onSelectAll,
  isOpen,
  embedded = false
}: TaskFilterProps) {
  const statusConfig = {
    pending: { label: "Pendente", bg: "bg-[#18181B]", text: "text-white" },
    in_progress: { label: "Andamento", bg: "bg-[#17FF8B]", text: "text-black" },
    paused: { label: "Pausada", bg: "bg-[#F4F4F5]", text: "text-[#FF3366]" },
    waiting: { label: "Atrasando", bg: "bg-[#FFCC00]", text: "text-black" },
    delayed: { label: "Atrasada", bg: "bg-[#FF3366]", text: "text-white" },
    completed: { label: "Concluído", bg: "bg-[#8B5CF6]", text: "text-white" }
  }

  const handleSelectAll = () => {
    onSelectAll(Object.keys(statusConfig) as Task["status"][])
  }

  if (!isOpen) return null

  const containerClass = embedded
    ? "w-full h-full flex flex-col bg-[#333333] p-4"
    : "fixed z-50 bg-[#333333] backdrop-blur-sm rounded-2xl shadow-lg w-90 p-6 flex flex-col"

  return (
    <div className={containerClass}>
      {/* HEADER */}
      <div
        style={{ marginTop: "15px", textAlign: "center" }}
        className="flex flex-line items-center justify-center gap-2"
      >
        <Filter className="w-4 h-4 text-white" strokeWidth={2} />
        <span className="text-xs font-bold text-white tracking-wide uppercase">
          Filtro de Tarefas
        </span>
      </div>

      {/* BOTÕES DE AÇÃO */}
      <div style={{ padding: "16px", top: 10, left: 10 }} className="flex items-center gap-4 mb-4">
        <button
          onClick={onClear}
          className="flex-1 h-6 bg-[#7F7F7F] hover:bg-[#666666] text-white text-[10px] font-bold rounded-full uppercase transition-colors"
        >
          Limpar
        </button>
        <button
          onClick={handleSelectAll}
          className="flex-1 h-6 bg-[#17FF8B] hover:bg-[#12cc6f] text-black text-[10px] font-bold rounded-full uppercase transition-colors shadow-[0_0_10px_rgba(23,255,139,0.3)]"
        >
          Todos
        </button>
      </div>

      {/* LISTA DE STATUS */}
      <div className="flex flex-col gap-2 flex-1 overflow-y-auto custom-scrollbar items-center">
        {Object.entries(statusConfig).map(([status, config]) => {
          const taskStatus = status as Task["status"]
          const isSelected = selectedFilters.has(taskStatus)

          return (
            <button
              key={status}
              onClick={() => onToggleFilter(taskStatus)}
              className={`w-[90%] h-6 text-[10px] font-bold uppercase transition-all px-4 flex items-center tracking-wider shadow-sm
                ${
                  isSelected
                    ? `${config.bg} ${config.text}`
                    : "bg-[#7F7F7F] text-white hover:bg-[#8f8f8f]"
                }
              `}
            >
              <span style={{ width: "10px", textAlign: "left", marginLeft: 20 }}>
                {config.label}
              </span>
            </button>
          )
        })}
      </div>

      <div style={{ marginBottom: "15px", textAlign: "center" }}>
        <span className="text-[10px] text-[#f2f2f2] font-light uppercase tracking-wider">
          {selectedFilters.size} Filtros Ativos
        </span>
      </div>
    </div>
  )
}
