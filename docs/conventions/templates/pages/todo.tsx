export const page = {
  head: {
    title: "Zebflow Todo",
  },
  navigation: "history",
};

export const app = (() => {
function syncView(ctx) {
    const items = Array.isArray(ctx.get("todo.items")) ? ctx.get("todo.items") : [];
    const count = items.length;
    ctx.set("todo.count", count);
    ctx.set("todo.isEmpty", count === 0);
    ctx.set("todo.listText", items.map((v, i) => `${i + 1}. ${v}`).join("\n"));
  }

  return {
    state: {
      todo: {
        draft: "",
        items: ["Ship V1"],
        count: 1,
        isEmpty: false,
        listText: "1. Ship V1"
      }
    },
    actions: {
      "todo.add": (ctx) => {
        const draft = String(ctx.get("todo.draft") || "").trim();
        if (!draft) return "todo.draft";
        const items = Array.isArray(ctx.get("todo.items")) ? ctx.get("todo.items").slice() : [];
        items.push(draft);
        ctx.set("todo.items", items);
        ctx.set("todo.draft", "");
        return "todo.items";
      },
      "todo.clear": (ctx) => {
        ctx.set("todo.items", []);
        return "todo.items";
      }
    },
    memo: {
      "todo.preview": (ctx) => String(ctx.get("todo.draft") || "").trim()
    },
    effect: {
      "todo.sync": {
        deps: ["todo.items"],
        immediate: true,
        run: syncView
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <main className="px-4 py-2">
      <h1>Todo</h1>
      <p>Total: <span jText="todo.count">0</span></p>

      <label>
        New item
        <input jModel="todo.draft" />
      </label>
      <button onClick="todo.add">Add</button>
      <button onClick="todo.clear">Clear</button>

      <p jShow="todo.isEmpty">No items yet.</p>
      <p jHide="todo.isEmpty">Items available.</p>

      <pre jText="todo.listText"></pre>
    </main>
</Page>
  );
}
