import { useState, useMemo, useEffect, useRef, usePageState, useNavigate, Link } from "zeb";

// ─── Inline sub-components ────────────────────────────────────────────────

function NavBar({ active }: any) {
  return (
    <nav class="bg-zinc-900 border-b border-zinc-800 px-6 py-3 flex items-center justify-between sticky top-0 z-10">
      <span class="text-zinc-100 font-bold text-sm tracking-tight">RWE Demo</span>
      <div class="flex gap-5 text-sm">
        <Link href="/" class={active === "/" ? "text-emerald-400 font-medium" : "text-zinc-400 hover:text-zinc-100"}>Home</Link>
        <Link href="/blog" class={active === "/blog" ? "text-emerald-400 font-medium" : "text-zinc-400 hover:text-zinc-100"}>Blog</Link>
        <Link href="/todo" class={active === "/todo" ? "text-emerald-400 font-medium" : "text-zinc-400 hover:text-zinc-100"}>Todo</Link>
      </div>
    </nav>
  );
}

function TodoItem({ id, text, completed, onToggle, onDelete }: any) {
  return (
    <li class="flex items-center gap-3 px-4 py-3 border-b border-zinc-800 last:border-0 group hover:bg-zinc-900/60">
      <button
        type="button"
        onClick={() => onToggle(id)}
        class={
          completed
            ? "w-5 h-5 rounded-full border border-emerald-500 bg-emerald-500 flex-shrink-0"
            : "w-5 h-5 rounded-full border border-zinc-600 bg-transparent flex-shrink-0 hover:border-emerald-400"
        }
      />
      <span
        class={
          completed
            ? "flex-grow text-sm text-zinc-500 line-through"
            : "flex-grow text-sm text-zinc-100"
        }
      >
        {text}
      </span>
      <button
        type="button"
        onClick={() => onDelete(id)}
        class="text-xs text-zinc-600 group-hover:text-zinc-400 hover:text-red-400 px-2 py-1 rounded border border-transparent hover:border-red-800 transition-colors"
      >
        ✕
      </button>
    </li>
  );
}

// ─── Page ─────────────────────────────────────────────────────────────────

export const page = {
  head: { title: "Todo — RWE Demo" },
  html: { lang: "en" },
};

export default function TodoPage(input: any) {
  // Seed items from the server-rendered input
  const seedItems = (input.items || []).map((it: any, i: number) => ({
    id: i + 1,
    text: it.title ?? it.text ?? String(it),
    completed: false,
  }));

  // useState
  const [todos, setTodos] = useState<any[]>(seedItems);
  const [inputValue, setInputValue] = useState<string>("");
  const [filter, setFilter] = useState<string>("all");

  // useRef — focused on mount (browser only)
  const inputRef = useRef<any>(null);

  // usePageState — records the last added item, visible across all components
  const { lastAdded = "", setPageState } = usePageState({ lastAdded: "" });

  // useNavigate
  const navigate = useNavigate();

  // useMemo — derived counts
  const activeCount = useMemo(() => todos.filter((t: any) => !t.completed).length, [todos]);
  const doneCount = useMemo(() => todos.filter((t: any) => t.completed).length, [todos]);

  // useMemo — filtered list
  const filtered = useMemo(() => {
    if (filter === "active") return todos.filter((t: any) => !t.completed);
    if (filter === "done") return todos.filter((t: any) => t.completed);
    return todos;
  }, [todos, filter]);

  // useEffect — auto-focus input on mount
  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  const addTodo = (e: any) => {
    e?.preventDefault?.();
    const text = String(inputValue || "").trim();
    if (!text) return;
    const id = Date.now();
    setTodos((prev: any[]) => [{ id, text, completed: false }, ...prev]);
    setInputValue("");
    setPageState({ lastAdded: text });
  };

  const toggle = (id: number) => {
    setTodos((prev: any[]) =>
      prev.map((t: any) => (t.id === id ? { ...t, completed: !t.completed } : t))
    );
  };

  const remove = (id: number) => {
    setTodos((prev: any[]) => prev.filter((t: any) => t.id !== id));
  };

  const clearDone = () => {
    setTodos((prev: any[]) => prev.filter((t: any) => !t.completed));
  };

  return (
    <div class="min-h-screen bg-zinc-950 text-zinc-100 flex flex-col">
      <NavBar active="/todo" />

      <main class="flex-1 max-w-2xl mx-auto w-full px-6 py-10">

        {/* Hero */}
        <div class="mb-8">
          <p class="text-xs font-mono text-emerald-400 uppercase tracking-widest mb-1">Todo</p>
          <h1 class="text-3xl font-bold text-zinc-100 mb-2">Task Manager</h1>
          <p class="text-zinc-400 text-sm">
            useState · useMemo · useEffect · useRef · usePageState · TodoItem component
          </p>
          {lastAdded !== "" && (
            <p class="mt-2 text-xs text-emerald-500 font-mono">
              Last added: "{lastAdded}"
            </p>
          )}
        </div>

        {/* Add form (useState for input, useEffect focuses on mount) */}
        <form onSubmit={addTodo} class="flex gap-2 mb-6">
          <input
            ref={inputRef}
            type="text"
            value={inputValue}
            onInput={(e: any) => setInputValue(e?.target?.value ?? "")}
            placeholder="What needs doing?"
            class="flex-1 bg-zinc-900 border border-zinc-700 rounded-lg px-4 py-2.5 text-sm text-zinc-100 placeholder-zinc-600 focus:border-emerald-500 focus:outline-none"
          />
          <button
            type="submit"
            class="px-5 py-2.5 bg-emerald-900 hover:bg-emerald-800 border border-emerald-700 rounded-lg text-sm font-medium text-emerald-100 disabled:opacity-40"
          >
            Add
          </button>
        </form>

        {/* Stats (useMemo) */}
        <div class="flex gap-5 mb-4 text-xs text-zinc-500">
          <span>Active: <strong class="text-zinc-300 font-mono">{activeCount}</strong></span>
          <span>Done: <strong class="text-zinc-300 font-mono">{doneCount}</strong></span>
          <span>Total: <strong class="text-zinc-300 font-mono">{todos.length}</strong></span>
        </div>

        {/* Filter tabs (useState) */}
        {todos.length > 0 && (
          <div class="flex gap-1 mb-4 bg-zinc-900 border border-zinc-800 rounded-lg p-1 w-fit">
            {["all", "active", "done"].map((f) => (
              <button
                key={f}
                onClick={() => setFilter(f)}
                class={
                  filter === f
                    ? "px-3 py-1.5 rounded-md bg-zinc-700 text-zinc-100 text-xs font-medium"
                    : "px-3 py-1.5 rounded-md text-zinc-500 text-xs hover:text-zinc-300"
                }
              >
                {f}
              </button>
            ))}
          </div>
        )}

        {/* Todo list — TodoItem sub-component */}
        <div class="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden mb-3">
          {filtered.length === 0 ? (
            <div class="py-12 text-center text-zinc-500 text-sm">
              {todos.length === 0
                ? "No tasks yet — add one above."
                : "No tasks match this filter."}
            </div>
          ) : (
            <ul>
              {filtered.map((todo: any) => (
                <TodoItem
                  key={String(todo.id)}
                  id={todo.id}
                  text={todo.text}
                  completed={todo.completed}
                  onToggle={toggle}
                  onDelete={remove}
                />
              ))}
            </ul>
          )}
        </div>

        {/* Clear completed */}
        {doneCount > 0 && (
          <button
            onClick={clearDone}
            class="text-xs text-zinc-500 hover:text-red-400 transition-colors"
          >
            Clear {doneCount} completed task{doneCount !== 1 ? "s" : ""}
          </button>
        )}

        {/* useNavigate demo button */}
        <div class="mt-8 pt-6 border-t border-zinc-800">
          <p class="text-xs text-zinc-500 mb-3">Navigate with useNavigate:</p>
          <div class="flex gap-2 mb-5">
            <button
              onClick={() => navigate("/")}
              class="px-3 py-1.5 bg-zinc-800 border border-zinc-700 hover:bg-zinc-700 rounded text-xs text-zinc-300"
            >
              navigate("/")
            </button>
            <button
              onClick={() => navigate("/blog")}
              class="px-3 py-1.5 bg-zinc-800 border border-zinc-700 hover:bg-zinc-700 rounded text-xs text-zinc-300"
            >
              navigate("/blog")
            </button>
          </div>
          <div class="flex gap-4 text-sm">
            <Link href="/" class="text-zinc-400 hover:text-zinc-200 no-underline">← Home</Link>
            <Link href="/blog" class="text-zinc-400 hover:text-zinc-200 no-underline">Blog →</Link>
          </div>
        </div>

      </main>

      <footer class="bg-zinc-900 border-t border-zinc-800 px-6 py-4 mt-auto">
        <div class="max-w-2xl mx-auto flex items-center justify-between text-xs text-zinc-500">
          <span>RWE — Reactive Web Engine</span>
          <div class="flex gap-4">
            <Link href="/" class="hover:text-zinc-300 no-underline">Home</Link>
            <Link href="/blog" class="hover:text-zinc-300 no-underline">Blog</Link>
            <Link href="/todo" class="hover:text-zinc-300 no-underline">Todo</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
