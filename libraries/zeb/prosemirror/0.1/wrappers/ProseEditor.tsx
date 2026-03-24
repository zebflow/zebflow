/**
 * ProseEditor — RWE wrapper for ProseMirror rich text editor.
 *
 * Accepts individual typed props (TipTap-style) and serialises them to
 * data-config so the bundle's MutationObserver can mount the editor.
 *
 * IMPORT in TSX page:
 *   import ProseEditor from "zeb/prosemirror";
 *
 * ─── SPA / API-first pattern (primary) ───────────────────────────────────
 *   const [body, setBody] = usePageState("body", "");
 *   useEffect(() => {
 *     fetch("/api/post/1").then(r => r.json()).then(d => setBody(d.html));
 *   }, []);
 *   <ProseEditor stateKey="body" toolbar="full" className="min-h-[400px]" />
 *
 *   On each edit, the editor calls window.__rweSetPageState({ body: html })
 *   so `body` in page state is always current. On submit: just read `body`.
 *
 * ─── Read-only list from API ──────────────────────────────────────────────
 *   {submissions.map(s => (
 *     <ProseEditor
 *       key={s.id}
 *       id={`preview-${s.id}`}
 *       content={s.answer}   // set at Preact render time (post-fetch)
 *       editable={false}
 *       toolbar={false}
 *     />
 *   ))}
 *   MutationObserver auto-mounts these as they appear in the DOM.
 *
 * ─── Examiner swapping (single editor, content driven by page state) ──────
 *   const [current, setCurrent] = usePageState("current", "");
 *   setCurrent(submissions[idx].answer);   // drives the editor reactively
 *   <ProseEditor stateKey="current" editable={false} toolbar={false} />
 *
 * ─── Multi-editor assessment ──────────────────────────────────────────────
 *   {questions.map(q => (
 *     <ProseEditor
 *       key={q.id}
 *       id={`ans-${q.id}`}
 *       stateKey={`ans_${q.id}`}
 *       statsKey={`stats_${q.id}`}
 *       toolbar="basic"
 *       placeholder="Write your answer..."
 *     />
 *   ))}
 *   On submit: window.__zebProse.get(`ans-${q.id}`).getHTML()
 *
 * ─── Custom toolbar plugins ───────────────────────────────────────────────
 *   // Register in a behavior file:
 *   window.__zebProse.registerPlugin({
 *     id: "ai-improve", label: "AI", icon: "<svg>…</svg>",
 *     async onActivate(ctx) {
 *       const improved = await callMyApi(ctx.getSelectedText());
 *       ctx.replaceSelection(improved);
 *     }
 *   });
 *   // Use in toolbar:
 *   <ProseEditor toolbar={["bold", "italic", "|", "ai-improve"]} stateKey="body" />
 *
 * ─── Imperative access ────────────────────────────────────────────────────
 *   const inst = window.__zebProse.get("my-editor");
 *   inst.setHTML("<p>replaced</p>");
 *   inst.getHTML();   // → "<p>replaced</p>"
 *   inst.focus();
 */
export const app = {};

export default function ProseEditor(props) {
  /*
   * Build the config object from individual named props and bake it into
   * data-config as JSON.  The bundle reads this attribute at mount time.
   *
   * Content priority at mount:
   *   1. window.__rwePageState[stateKey]  — if stateKey is set and has data
   *   2. content prop                     — set at Preact render time
   *   3. empty                            — start fresh (user will type)
   */
  const config = JSON.stringify({
    content:     props.content,
    stateKey:    props.stateKey,
    statsKey:    props.statsKey,
    editable:    props.editable !== false,
    autofocus:   props.autofocus ?? false,
    placeholder: props.placeholder,
    toolbar:     props.toolbar !== undefined ? props.toolbar : "basic",
    toolbarMode: props.toolbarMode ?? "inline",
  });

  return (
    <div
      data-zeb-lib="prosemirror"
      data-zeb-wrapper="ProseEditor"
      data-config={config}
      id={props.id}
      className={props.className || "w-full min-h-[200px]"}
      hydrate="visible"
    />
  );
}
