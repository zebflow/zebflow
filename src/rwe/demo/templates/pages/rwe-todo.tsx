import { useState } from "zeb";

export default function TodoApp() {
  const [todos, setTodos] = useState([]);
  const [inputValue, setInputValue] = useState("");
  const [filter, setFilter] = useState("all");

  const addTodo = (event) => {
    event?.preventDefault?.();
    const text = String(inputValue || "").trim();
    if (!text) return;

    setTodos((prev) => [{ id: Date.now(), text, completed: false }, ...prev]);
    setInputValue("");
  };

  const toggleTodo = (id) => {
    setTodos((prev) =>
      prev.map((todo) => (todo.id === id ? { ...todo, completed: !todo.completed } : todo)),
    );
  };

  const deleteTodo = (id) => {
    setTodos((prev) => prev.filter((todo) => todo.id !== id));
  };

  const clearCompleted = () => {
    setTodos((prev) => prev.filter((todo) => !todo.completed));
  };

  const filteredTodos = todos.filter((todo) => {
    if (filter === "active") return !todo.completed;
    if (filter === "completed") return todo.completed;
    return true;
  });

  const activeTodosCount = todos.filter((todo) => !todo.completed).length;
  const completedTodosCount = todos.filter((todo) => todo.completed).length;

  return (
    <div className="min-h-screen bg-gray-950 px-6 py-10 font-mono text-gray-100">
      <div className="mx-auto max-w-4xl rounded-2xl border border-gray-800 bg-gradient-to-br from-gray-900 to-gray-950 shadow-2xl">
        <div className="border-b border-gray-800 px-6 py-5">
          <p className="text-xs uppercase tracking-[0.22em] text-indigo-300">rwe</p>
          <h1 className="mt-2 text-3xl font-bold text-indigo-400">Task Console</h1>
          <p className="mt-1 text-sm text-gray-400">Comprehensive-inspired dense workspace layout.</p>
        </div>

        <div className="px-6 py-5">
          <div className="mb-5 rounded-xl border border-gray-800 bg-black/30 p-4">
            <form onSubmit={addTodo} className="relative">
              <input
                type="text"
                value={inputValue}
                onInput={(event) => setInputValue(event?.target?.value || "")}
                placeholder="What needs to be done?"
                className="w-full rounded-lg border border-gray-700 bg-gray-900 px-4 py-3 pr-14 text-sm text-gray-100 placeholder-gray-500 focus:border-indigo-500 focus:outline-none"
              />
              <button
                type="submit"
                disabled={!String(inputValue || "").trim()}
                className="absolute bottom-1.5 right-1.5 top-1.5 rounded-md border border-indigo-500 bg-indigo-600/30 px-3 text-sm font-semibold text-indigo-100 hover:bg-indigo-600/50 disabled:cursor-not-allowed disabled:border-gray-700 disabled:bg-gray-800 disabled:text-gray-500"
              >
                +
              </button>
            </form>
          </div>

          {todos.length > 0 ? (
            <div className="mb-4 flex items-center justify-start gap-2 rounded-lg border border-gray-800 bg-black/30 p-2">
              {["all", "active", "completed"].map((nextFilter) => (
                <button
                  key={nextFilter}
                  type="button"
                  onClick={() => setFilter(nextFilter)}
                  className={
                    filter === nextFilter
                      ? "rounded-md border border-indigo-500 bg-indigo-600/30 px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-indigo-100"
                      : "rounded-md border border-gray-700 bg-gray-900 px-3 py-1.5 text-xs font-semibold uppercase tracking-wide text-gray-400 hover:text-gray-200"
                  }
                >
                  {nextFilter}
                </button>
              ))}
            </div>
          ) : null}

          <div className="overflow-hidden rounded-xl border border-gray-800">
            {filteredTodos.length === 0 ? (
              <div className="bg-black/30 p-10 text-center text-sm text-gray-500">No tasks yet</div>
            ) : (
              <ul className="max-h-[420px] divide-y divide-gray-800 overflow-y-auto bg-black/30">
                {filteredTodos.map((todo) => (
                  <li
                    key={String(todo.id)}
                    className="group flex items-center gap-3 px-4 py-3 text-sm hover:bg-gray-900/80"
                  >
                    <button
                      type="button"
                      onClick={() => toggleTodo(todo.id)}
                      className={
                        todo.completed
                          ? "h-5 w-5 flex-shrink-0 rounded-full border border-green-500 bg-green-500"
                          : "h-5 w-5 flex-shrink-0 rounded-full border border-gray-600 bg-transparent hover:border-indigo-400"
                      }
                    />
                    <span
                      className={
                        todo.completed
                          ? "flex-grow text-gray-500 line-through"
                          : "flex-grow text-gray-100"
                      }
                    >
                      {todo.text}
                    </span>
                    <button
                      type="button"
                      onClick={() => deleteTodo(todo.id)}
                      className="rounded-md border border-gray-700 bg-gray-900 px-2 py-1 text-xs font-semibold text-gray-400 hover:border-red-500 hover:text-red-300"
                    >
                      Delete
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {todos.length > 0 ? (
            <div className="mt-4 flex items-center justify-between rounded-lg border border-gray-800 bg-black/30 px-4 py-3 text-xs text-gray-400">
              <span>
                Active: <strong className="text-gray-200">{activeTodosCount}</strong> · Completed:{" "}
                <strong className="text-gray-200">{completedTodosCount}</strong>
              </span>
              {completedTodosCount > 0 ? (
                <button
                  type="button"
                  onClick={clearCompleted}
                  className="rounded-md border border-gray-700 bg-gray-900 px-2 py-1 text-xs font-semibold text-gray-400 hover:border-red-500 hover:text-red-300"
                >
                  Clear completed
                </button>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
