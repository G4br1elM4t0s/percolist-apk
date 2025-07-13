import { useState, useEffect, forwardRef } from "react"
import { useTaskStore, type Task } from "../store/task.store"
import { TaskButton } from "./TaskButton"
import { Search, X } from "lucide-react"

interface TaskListViewProps {
  isOpen: boolean
  onClose: () => void
  anchorRef: React.RefObject<HTMLButtonElement | null>
}

export const TaskListView = forwardRef<HTMLDivElement, TaskListViewProps>(
  ({ isOpen, anchorRef }, ref) => {
    const { tasks, loadTasks } = useTaskStore()
    const [filteredTasks, setFilteredTasks] = useState<Task[]>(tasks)
    const [searchTerm, setSearchTerm] = useState("")

    const [position, setPosition] = useState({
      top: 0,
      left: 0
    })

    useEffect(() => {
      if (isOpen && anchorRef.current) {

        const rect = anchorRef.current.getBoundingClientRect();
        const width = 572; // mesma largura do seu componente
        const left = rect.left - 50 + rect.width / 2 - width / 2;

        setPosition({
          top: rect.bottom + 8,
          left
        });
      }
    }, [isOpen, !!anchorRef])

    useEffect(() => {
      setFilteredTasks(tasks)
    }, [tasks])

    useEffect(() => {
      if (isOpen) {
        loadTasks()
      }
    }, [isOpen, loadTasks])

    // Aplicar filtro de busca por nome
    const searchFilteredTasks = filteredTasks.filter(task =>
      task.name.toLowerCase().includes(searchTerm.toLowerCase())
    )

    if (!isOpen) return null
    const isActive = searchTerm.length > 0;

    return (
      <div
        ref={ref}
        style={{
          padding: "16px",
          top: position.top,
          left: position.left,
        }}
        className="fixed right-0 bottom-[60px] z-40 p-4 w-[572px] h-[284px] bg-[#1A1A1A] rounded-lg shadow-xl border border-[#2A2A2A] overflow-hidden"
      >
        <div className="flex flex-col h-full gap-4">
          {/* Barra de busca */}
          <div
          style={{
             padding: "4px",
          }}
            className={`flex items-center w-full  rounded-full transition-colors bg-[#444444] border ${
              isActive ? 'border-[#17FF8B]' : 'border-[#2A2A2A]'
            }`}
      >
        <Search style={{marginRight:"8px", marginLeft:"8px"}} className="w-4 h-4 text-gray-400 " />

        <input
          type="text"
          placeholder="Buscar tarefas por nome..."
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="flex-1 bg-transparent outline-none text-white placeholder-gray-400 text-sm"
        />

        {isActive && (
          <button
            onClick={() => setSearchTerm("")}
            className="text-gray-400 hover:text-white ml-2"
          >
            <X className="w-4 h-4" />
          </button>
        )}
      </div>

          <div className="flex-1 overflow-y-auto custom-scrollbar">
            {searchFilteredTasks.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-gray-400">
                <div className="text-6xl mb-4">📝</div>
                <h3 className="text-lg font-medium mb-2">Nenhuma tarefa encontrada</h3>
                <p className="text-sm text-center">
                  {searchTerm
                    ? `Nenhuma tarefa corresponde à busca "${searchTerm}"`
                    : "Tente ajustar os filtros ou criar uma nova tarefa"}
                </p>
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-4 pr-2">
                {searchFilteredTasks.map((task, index) => (
                    <TaskButton
                      key={task.id}
                      task={task}
                      index={index}
                      isListView={true}
                      listViewAnchorRef={
                        ref && typeof ref !== "function"
                          ? (ref as React.RefObject<HTMLDivElement>)
                          : undefined
                      }
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

TaskListView.displayName = 'TaskListView'
