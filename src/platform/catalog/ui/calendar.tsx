import { cx } from "zeb";
import { useState } from "zeb";

interface CalendarProps {
  selected?: Date;
  onSelect?: (date: Date) => void;
  defaultMonth?: Date;
  disabled?: (date: Date) => boolean;
  className?: string;
  [key: string]: any;
}

const MONTHS = ["January","February","March","April","May","June","July","August","September","October","November","December"];
const DAYS = ["Su","Mo","Tu","We","Th","Fr","Sa"];

function getDaysInMonth(year: number, month: number) {
  return new Date(year, month + 1, 0).getDate();
}

function getFirstDayOfMonth(year: number, month: number) {
  return new Date(year, month, 1).getDay();
}

function isSameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function Calendar({ selected, onSelect, defaultMonth, disabled, className, ...rest }: CalendarProps) {
  const today = new Date();
  const [viewDate, setViewDate] = useState(defaultMonth ?? selected ?? today);
  const year = viewDate.getFullYear();
  const month = viewDate.getMonth();

  const prevMonth = () => setViewDate(new Date(year, month - 1, 1));
  const nextMonth = () => setViewDate(new Date(year, month + 1, 1));

  const daysInMonth = getDaysInMonth(year, month);
  const firstDay = getFirstDayOfMonth(year, month);
  const totalCells = Math.ceil((firstDay + daysInMonth) / 7) * 7;

  const cells = Array.from({ length: totalCells }, (_, i) => {
    const dayNum = i - firstDay + 1;
    if (dayNum < 1 || dayNum > daysInMonth) return null;
    return new Date(year, month, dayNum);
  });

  return (
    <div className={cx("p-3 space-y-4", className)} {...rest}>
      {/* Month navigation */}
      <div className="relative flex items-center justify-between pt-1">
        <button
          type="button"
          onClick={prevMonth}
          className="inline-flex items-center justify-center h-7 w-7 rounded-md border border-gray-200 bg-transparent p-0 hover:bg-gray-100"
          aria-label="Previous month"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
            <path d="m15 18-6-6 6-6" />
          </svg>
        </button>
        <div className="text-sm font-medium">{MONTHS[month]} {year}</div>
        <button
          type="button"
          onClick={nextMonth}
          className="inline-flex items-center justify-center h-7 w-7 rounded-md border border-gray-200 bg-transparent p-0 hover:bg-gray-100"
          aria-label="Next month"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="h-4 w-4">
            <path d="m9 18 6-6-6-6" />
          </svg>
        </button>
      </div>

      {/* Calendar grid */}
      <table className="w-full border-collapse space-y-1">
        <thead>
          <tr className="flex">
            {DAYS.map(d => (
              <th key={d} className="text-gray-500 rounded-md w-9 font-normal text-[0.8rem]">{d}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {Array.from({ length: totalCells / 7 }, (_, week) => (
            <tr key={week} className="flex w-full mt-2">
              {cells.slice(week * 7, (week + 1) * 7).map((date, i) => {
                if (!date) return <td key={i} className="h-9 w-9 text-center text-sm p-0" />;
                const isSelected = selected ? isSameDay(date, selected) : false;
                const isToday = isSameDay(date, today);
                const isDisabled = disabled?.(date) ?? false;
                return (
                  <td key={i} className="h-9 w-9 text-center text-sm p-0">
                    <button
                      type="button"
                      onClick={() => !isDisabled && onSelect?.(date)}
                      disabled={isDisabled}
                      className={cx(
                        "inline-flex h-9 w-9 items-center justify-center rounded-md text-sm font-normal transition-colors",
                        isSelected ? "bg-gray-900 text-white hover:bg-gray-800" :
                        isToday ? "bg-gray-100 text-gray-900 font-medium" :
                        "hover:bg-gray-100 hover:text-gray-900",
                        isDisabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"
                      )}
                    >
                      {date.getDate()}
                    </button>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export default Calendar;
