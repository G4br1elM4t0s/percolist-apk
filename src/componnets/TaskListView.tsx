import { useState, useEffect, forwardRef, useRef } from "react"
import { useTaskStore, type Task } from "../store/task.store"
import { TaskButton } from "./TaskButton"
import { FilterIcon } from "../components/FilterIcon"
import { Search, X } from "lucide-react"
import { TaskFilter } from "./TaskFilter"

interface TaskListViewProps {
  isOpen: boolean
  onClose: () => void
  anchorRef: React.RefObject<HTMLButtonElement | null>
}

export const TaskListView = forwardRef<HTMLDivElement, TaskListViewProps>(
  ({ isOpen, anchorRef }, ref) => {
    const { tasks, loadTasks } = useTaskStore()
    const [filteredTasks, setFilteredTasks] = useState<Task[]>(tasks)
    
    const [selectedFilters, setSelectedFilters] = useState<Set<Task["status"]>>(new Set())
    
    const [searchTerm, setSearchTerm] = useState("")
    const [isFilterOpen, setIsFilterOpen] = useState(false)
    const filterRef = useRef<HTMLDivElement>(null)
    const filterButtonRef = useRef<HTMLDivElement>(null)

    const [position, setPosition] = useState({ top: 0, left: 0 })

    useEffect(() => {
      if (isOpen && anchorRef.current) {
        const rect = anchorRef.current.getBoundingClientRect()
        const width = 572
        const left = rect.left - 50 + rect.width / 2 - width / 2
        setPosition({ top: rect.bottom + 8, left })
      }
    }, [isOpen, !!anchorRef])

    
    useEffect(() => {
      if (selectedFilters.size === 0) {
        setFilteredTasks(tasks)
      } else {
        const newFiltered = tasks.filter(task => selectedFilters.has(task.status))
        setFilteredTasks(newFiltered)
      }
    }, [tasks, selectedFilters])
    

    useEffect(() => {
      if (isOpen) {
        loadTasks()
      }
    }, [isOpen, loadTasks])

    useEffect(() => {
      if (!isOpen) {
        setIsFilterOpen(false)
        setSearchTerm("")
      }
    }, [isOpen])

    useEffect(() => {
      function handleClickOutside(event: MouseEvent) {
        if (!isFilterOpen) return
        const target = event.target as Node
        if (filterRef.current && filterRef.current.contains(target)) return
        if (filterButtonRef.current && filterButtonRef.current.contains(target)) return
        setIsFilterOpen(false)
      }
      document.addEventListener("mousedown", handleClickOutside)
      return () => document.removeEventListener("mousedown", handleClickOutside)
    }, [isFilterOpen])

    const toggleFilter = (status: Task["status"]) => {
      const newFilters = new Set(selectedFilters)
      if (newFilters.has(status)) newFilters.delete(status)
      else newFilters.add(status)
      setSelectedFilters(newFilters)
    }

    const clearFilters = () => {
      setSelectedFilters(new Set())
    }

    const selectAllFilters = (allStatuses: Task["status"][]) => {
      setSelectedFilters(new Set(allStatuses))
    }

    // Busca por texto 
    const searchFilteredTasks = filteredTasks.filter(task =>
      task.name.toLowerCase().includes(searchTerm.toLowerCase())
    )

    if (!isOpen) return null
    const isActive = searchTerm.length > 0

    return (
      <div
        ref={ref}
        style={{ padding: "16px", top: position.top, left: position.left }}
        className="fixed right-0 bottom-[60px] z-40 p-4 w-[572px] h-[384px] bg-[#1A1A1A] rounded-lg shadow-xl border border-[#2A2A2A] overflow-hidden"
      >
        {isFilterOpen && (
          <div 
            ref={filterRef} 
            className="absolute top-[50px] left-4 bottom-4 w-[360px] z-50 bg-[#333333] rounded-xl shadow-2xl border border-zinc-700 animate-in slide-in-from-left-2 duration-200 overflow-hidden"
          >
            <TaskFilter 
              isOpen={true} 
              onClose={() => setIsFilterOpen(false)}
              embedded={true}
              
              selectedFilters={selectedFilters}
              onToggleFilter={toggleFilter}
              onClear={clearFilters}
              onSelectAll={selectAllFilters}
            />
          </div>
        )}

        <div className="flex flex-col h-full gap-4">
          <div className="flex items-center gap-4">
            <div
              ref={filterButtonRef}
              className="relative w-8 h-8 flex items-center justify-center cursor-pointer select-none"
              onClick={() => setIsFilterOpen(!isFilterOpen)}
            >
              {/* Ícone muda de cor se tiver filtro ativo OU menu aberto */}
              <div className={`absolute inset-0 flex items-center justify-center transition-all duration-300 ${
                  isFilterOpen || selectedFilters.size > 0 ? "opacity-100 scale-100" : "opacity-0 scale-90"
                }`}>
                <FilterIcon className="w-4 h-4 text-[#17FF8B]" />
              </div>

              <div className={`absolute inset-0 flex items-center justify-center transition-all duration-300 ${
                  !isFilterOpen && selectedFilters.size === 0 ? "opacity-100 scale-100" : "opacity-0 scale-90"
                }`}>
                <FilterIcon className="w-4 h-4 text-[#F2F2F2]" />
              </div>
            </div>

            <div style={{ padding: "4px" }} className={`flex items-center w-full rounded-full transition-colors bg-[#444444] border ${isActive ? "border-[#17FF8B]" : "border-[#2A2A2A]"}`}>
              <Search style={{ marginRight: "8px", marginLeft: "8px" }} className="w-4 h-4 text-gray-400 " />
              <input
                type="text"
                placeholder="Buscar tarefas por nome..."
                value={searchTerm}
                onChange={e => setSearchTerm(e.target.value)}
                className="flex-1 bg-transparent outline-none text-white placeholder-gray-400 text-sm"
              />
              {isActive && (
                <button onClick={() => setSearchTerm("")} className="text-gray-400 hover:text-white ml-2">
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 overflow-y-auto custom-scrollbar">
            {searchFilteredTasks.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-gray-400">
                <div className="text-6xl mb-4">📝</div>
                <h3 className="text-lg font-medium mb-2">Nenhuma tarefa encontrada</h3>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-4 pr-2">
                {searchFilteredTasks.map((task, index) => (
                  <TaskButton
                    key={task.id}
                    task={task}
                    index={index}
                    isListView={true}
                    listViewAnchorRef={ref && typeof ref !== "function" ? (ref as React.RefObject<HTMLDivElement>) : undefined}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    )
  }
)
TaskListView.displayName = "TaskListView"