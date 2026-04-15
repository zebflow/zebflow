export const page = {
  head: {
    title: "Zebflow List Render",
  },
  navigation: "history",
};

export const app = (() => {
function viewRows(items, size) {
    const out = [];
    for (let i = 0; i < size; i += 1) out.push(items[i] || "");
    return out;
  }

  function syncRows(ctx) {
    const items = Array.isArray(ctx.get("list.items")) ? ctx.get("list.items") : [];
    ctx.set("list.rows", viewRows(items, 5));
    ctx.set("list.length", items.length);
    ctx.set("list.hasItems", items.length > 0);
  }

  return {
    state: {
      list: {
        seq: 4,
        items: ["alpha", "beta", "gamma"],
        rows: ["alpha", "beta", "gamma", "", ""],
        length: 3,
        hasItems: true
      }
    },
    actions: {
      "list.append": (ctx) => {
        const items = Array.isArray(ctx.get("list.items")) ? ctx.get("list.items").slice() : [];
        const seq = Number(ctx.get("list.seq") || 0) + 1;
        items.push(`item-${seq}`);
        ctx.set("list.seq", seq);
        ctx.set("list.items", items);
        return "list.items";
      },
      "list.rotate": (ctx) => {
        const items = Array.isArray(ctx.get("list.items")) ? ctx.get("list.items").slice() : [];
        if (items.length > 1) items.push(items.shift());
        ctx.set("list.items", items);
        return "list.items";
      }
    },
    effect: {
      "list.sync": {
        deps: ["list.items"],
        immediate: true,
        run: syncRows
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <main className="px-4 py-2">
      <h1>List Render</h1>
      <button onClick="list.append">Append</button>
      <button onClick="list.rotate">Rotate</button>

      <p>Length: <span zText="list.length">0</span></p>
      <p zShow="list.hasItems">Has items</p>
      <p zHide="list.hasItems">No items</p>

      <ul>
        <li zText="list.rows.0">-</li>
        <li zText="list.rows.1">-</li>
        <li zText="list.rows.2">-</li>
        <li zText="list.rows.3">-</li>
        <li zText="list.rows.4">-</li>
      </ul>
    </main>
</Page>
  );
}
