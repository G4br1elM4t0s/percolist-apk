import React, { useEffect } from "react"
import { DayPicker } from "react-day-picker"
import "react-day-picker/dist/style.css"
import "./Calendar.css"
import { ptBR } from "date-fns/locale"

interface CalendarProps {
  selected?: Date
  onSelect?: (date: Date | undefined) => void
  highlightedDates?: Date[]
  onUnmount?: () => void
}

const Calendar: React.FC<CalendarProps> = ({
  selected,
  onSelect,
  highlightedDates = [],
  onUnmount
}) => {
  const isDayHighlighted = (day: Date): boolean => {
    return highlightedDates.some(
      date =>
        date.getFullYear() === day.getFullYear() &&
        date.getMonth() === day.getMonth() &&
        date.getDate() === day.getDate()
    )
  }

  useEffect(() => {
    return () => {
      if (onUnmount) {
        onUnmount()
      }
    }
  }, [onUnmount])

  return (
    <div className="custom-calendar  select-none">
      <DayPicker
        mode="single"
        selected={selected}
        onSelect={onSelect}
        showOutsideDays
        className="custom-calendar-picker"
        locale={ptBR}
        modifiers={{
          highlighted: isDayHighlighted,
          selected: selected,
          today: new Date()
        }}
        modifiersClassNames={{
          highlighted: "has-task",
          selected: "custom-selected",
          today: "custom-today"
        }}
        formatters={{
          formatWeekdayName: weekday => {
            const weekdays = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"]
            return weekdays[weekday.getDay()]
          }
        }}
      />
    </div>
  )
}

export default Calendar
