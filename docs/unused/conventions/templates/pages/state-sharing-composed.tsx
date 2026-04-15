export const page = {
  head: {
    title: "State Sharing Tree",
  },
  navigation: "history",
};

export const app = (() => {
const seed = Number(((input || {}).shared || {}).seed || 0);

  function setValue(ctx, next, label) {
    ctx.set("shared.value", Number(next));
    ctx.set("shared.lastAction", String(label || "unknown"));
    return "shared.value";
  }

  return {
    state: {
      shared: {
        seed,
        value: seed,
        lastAction: "init",
        double: seed * 2
      }
    },
    actions: {
      "tree.c.inc": (ctx) => setValue(ctx, Number(ctx.get("shared.value") || 0) + 1, "inc"),
      "tree.c.dec": (ctx) => setValue(ctx, Number(ctx.get("shared.value") || 0) - 1, "dec"),
      "tree.f.reset": (ctx) => setValue(ctx, Number(ctx.get("shared.seed") || 0), "reset")
    },
    memo: {
      "shared.double.memo": (ctx) => Number(ctx.get("shared.value") || 0) * 2
    },
    effect: {
      "shared.double.sync": {
        deps: ["shared.value"],
        immediate: true,
        run: (ctx) => {
          ctx.set("shared.double", Number(ctx.get("shared.value") || 0) * 2);
        }
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <main>
      <h1>Shared Root State Across Components</h1>
      <p>SSR seed value: {input.shared.seed}</p>
      <TreeA />
    </main>
</Page>
  );
}
