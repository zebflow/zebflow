export const page = {
  head: {
    links: [
      { rel: "stylesheet", href: "https://assets.safe/app.css" },
      { rel: "stylesheet", href: "https://blocked.bad/evil.css" }
    ],
    scripts: [
      { src: "https://cdn.safe/runtime.js" },
      { src: "https://blocked.bad/evil.js" }
    ]
  },
  navigation: "history",
};

export const app = (() => {
function counterDelta(ctx, by) {
    const current = Number(ctx.get("client.count") || 0);
    ctx.set("client.count", current + Number(by || 0));
    return "client.count";
  }

  function counterMemo(ctx) {
    return Number(ctx.get("client.count") || 0) * 2;
  }

  function counterEffect(ctx) {
    ctx.set("client.lastEffect", Number(ctx.get("client.count") || 0));
  }

  return {
    state: {
      client: {
        title: "Lean TSX",
        dynamicClass: "count-badge",
        showInfo: true,
        count: 0,
        lastEffect: 0
      }
    },
    actions: {
      "counter.inc": (ctx) => counterDelta(ctx, 1),
      "counter.dec": (ctx) => counterDelta(ctx, -1)
    },
    memo: {
      "counter.double": counterMemo
    },
    effect: {
      "counter.track": {
        deps: ["client.count"],
        immediate: true,
        run: counterEffect
      }
    }
  };
})();

export default function Page(input) {
  return (
<Page>
    <button className="rounded-lg bg-zinc-100 text-zinc-900 px-4 py-2" onClick="counter.inc">
      Click
    </button>
    <h1 zText="client.title">Fallback</h1>
    <input zModel="client.title" />
    <div zAttrClass="client.dynamicClass"></div>
    <p zShow="client.showInfo">Visible when showInfo=true</p>
    <p zHide="client.showInfo">Visible when showInfo=false</p>
</Page>
  );
}
