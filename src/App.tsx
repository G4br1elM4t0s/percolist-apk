import { useEffect, useState, useRef } from "react"
import { useTaskStore } from "./store/task.store"
import { TaskFooter } from "./componnets/TaskFooter"
import { TaskModal } from "./componnets/TaskModal"
import { usePomodoroChecker } from "./hooks/usePomodoroChecker"
import { useAutoResize } from "./hooks/useAutoResize"
import { useAuthListener } from "./hooks/useAuthListener"
import { invoke } from "@tauri-apps/api/core"
import { check } from "@tauri-apps/plugin-updater"
import { ask, message } from "@tauri-apps/plugin-dialog"

import { useAuthReleaseAndBlock } from "./hooks/useAuthReleaseAndBlock"
import { View } from "lucide-react"

async function checkForAppUpdates(onUserClick: boolean = false) {
  try {
    const update = await check()
    if (update === null) {
      await message("Falha ao verificar atualizações.\nTente novamente mais tarde.", {
        title: "Erro",
        kind: "error",
        okLabel: "OK"
      })
      return
    } else if (update?.available) {
      const yes = await ask(
        `Atualização para ${update.version} disponível!\n\nNotas da versão: ${update.body}`,
        {
          title: "Atualização Disponível",
          kind: "info",
          okLabel: "Atualizar",
          cancelLabel: "Cancelar"
        }
      )
      if (yes) {
        await update.downloadAndInstall()
        // Reiniciar o app após a atualização
        await invoke("graceful_restart")
      }
    } else if (onUserClick) {
      await message("Você está na versão mais recente. Continue incrível!", {
        title: "Nenhuma Atualização Disponível",
        kind: "info",
        okLabel: "OK"
      })
    }
  } catch (err) {
    console.error("Erro ao verificar atualizações:", err)
    if (onUserClick) {
      await message("Erro ao verificar atualizações. Tente novamente mais tarde.", {
        title: "Erro",
        kind: "error",
        okLabel: "OK"
      })
    }
  }
}

function App() {
  const { loadTasks, loadTasksWithSessions } = useTaskStore()
  const [isModalOpen, setIsModalOpen] = useState(false)
  const buttonRef = useRef<HTMLButtonElement>(null)
  const hasInitialized = useRef(false)
  useAuthReleaseAndBlock()

  // Ref para ajuste automático de altura
  const contentRef = useAutoResize([isModalOpen])

  // Verificador automático de sessões Pomodoro
  usePomodoroChecker(5000) // Verifica a cada 5 segundos

  // Listener para eventos de autenticação
  useAuthListener()

  useEffect(() => {
    if (hasInitialized.current) return

    hasInitialized.current = true
    const init = async () => {
      await Promise.all([loadTasks(), loadTasksWithSessions()])
      // Verificar atualizações na inicialização
      await checkForAppUpdates()
    }
    init()
  }, [])

  const handleOpenModal = async () => {
    try {
      await invoke("expand_window_for_modal")
      setIsModalOpen(true)
    } catch (error) {
      console.error("Error opening modal:", error)
    }
  }

  const handleCloseModal = async () => {
    await invoke("reset_window_size")
    setIsModalOpen(false)
  }

  // Fechar o modal quando clicar fora
  useEffect(() => {
    const handleClickOutside = async (event: MouseEvent) => {
      const target = event.target as HTMLElement
      if (isModalOpen && !target.closest(".task-modal")) {
        await handleCloseModal()
      }
    }

    document.addEventListener("mousedown", handleClickOutside)
    return () => document.removeEventListener("mousedown", handleClickOutside)
  }, [isModalOpen])

  return (
    <div ref={contentRef} className="fixed top-0 left-0 right-0">
      <TaskFooter onAddClick={handleOpenModal} buttonRef={buttonRef} isModalOpen={isModalOpen} />
      <TaskModal isOpen={isModalOpen} onClose={handleCloseModal} anchorEl={buttonRef} />
    </div>
  )
}

export default App
