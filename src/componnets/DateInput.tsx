import { useState, useEffect, useRef } from "react"
import { format } from "date-fns"
import { ptBR } from "date-fns/locale"
import Calendar from "./Calendar"

import { CalendarIcon } from "../components/CalendarIcon"
import { CalendarStartIcon } from "../components/CalendarStartIcon"

interface DateInputProps {
  value: Date
  onChange: (date: Date) => void
}

export default function DateInput({ value, onChange }: DateInputProps) {
  const [isCalendarOpen, setIsCalendarOpen] = useState(false)
  const calendarRef = useRef<HTMLDivElement>(null)
  const buttonRef = useRef<HTMLDivElement>(null)

  const formattedDate = format(value, "dd/MM/yyyy", {
    locale: ptBR
  })

  const handleSelect = (date: Date | undefined) => {
    if (date) {
      onChange(date)
      setIsCalendarOpen(false)
    }
  }

  // Fechar calendário ao clicar fora
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        isCalendarOpen &&
        calendarRef.current &&
        buttonRef.current &&
        !calendarRef.current.contains(event.target as Node) &&
        !buttonRef.current.contains(event.target as Node)
      ) {
        setIsCalendarOpen(false)
      }
    }

    if (isCalendarOpen) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isCalendarOpen])

  // Handler para fechar ao clicar no overlay
  const handleOverlayClick = (event: React.MouseEvent) => {
    if (event.target === event.currentTarget) {
      setIsCalendarOpen(false)
    }
  }

  return (
    <div className="relative flex items-center gap-1" ref={buttonRef}>
      {isCalendarOpen && (
        <div 
          className="fixed inset-0 flex items-center justify-center z-50 bg-black/50"
          onClick={handleOverlayClick}
        >
          <div className="bg-white rounded-lg shadow-lg" ref={calendarRef}>
            <Calendar selected={value} onSelect={handleSelect} />
          </div>
        </div>
      )}

      <button type="button" className="cursor-pointer" onClick={() => setIsCalendarOpen(!isCalendarOpen)}>
        <div className="relative w-10 h-10">
          <div className={`absolute inset-0 transition-all duration-500 ${isCalendarOpen ? 'opacity-100 scale-100' : 'opacity-0 scale-90'}`}>
            <CalendarStartIcon className="w-10 h-10 text-[#17FF8B]" />
          </div>
          <div className={`absolute inset-0 transition-all duration-500 ${!isCalendarOpen ? 'opacity-100 scale-100' : 'opacity-0 scale-90'}`}>
            <CalendarIcon className="w-10 h-10 text-white" />
          </div>
        </div>
      </button>

      <button
        type="button"
        className="h-7 px-4 w-full bg-[#D9D9D9] rounded-full flex items-center justify-center cursor-pointer hover:bg-zinc-700/70 transition-colors"
        onClick={() => setIsCalendarOpen(!isCalendarOpen)}
      >
        <div className="flex items-center">
          <span className="text-[#181818] text-sm font-semibold">{formattedDate}</span>
        </div>
      </button>
    </div>
  )
}
