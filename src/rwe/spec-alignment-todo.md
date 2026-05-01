# RWE Spec Alignment TODO

Open decisions to resolve before normalizing the spec/docs:

1. Canonical rule for page config
- Should `export const page` officially use only `ctx`?
- Or should `input` also be documented as acceptable there?

2. Canonical rule for page body
- Should `export default function Page(input)` remain the only recommended pattern?
- Or should body examples also mention bare `ctx` regularly?

3. Moustache rule
- Should we state explicitly:
  - moustache is for template/body interpolation only
  - never for `export const page`
- Or should the wording be even stricter?

4. Static generation in spec
- Should the RWE spec now officially acknowledge:
  - `n.web.static.generate`
  - `n.web.docs.generate`
  - shared static artifact/site layer
- Or should node-level static generation remain outside the core RWE spec?

5. SSG terminology
- Should “SSG” mean:
  - node-driven static artifact generation
- Or should “SSG” be reserved only for future native `render: "ssg"` inside page config?

6. Hydration docs
- Should the spec be updated to reflect current implemented hydration/island behavior?
- Or should the spec stay conservative and only document what is publicly blessed?

7. Imported module rule
- Should docs explicitly say:
  - entry page globals like `<Page>` are guaranteed in entry templates
  - imported helper modules should not assume those globals

8. Static page head data source
- For static page templates, should docs say page config must read from `ctx`
- even though `globalThis.input` currently aliases it?

9. Cleanup scope
- Patch only:
  - `SPECIFICATION.md`
  - help docs
  - bad fixtures/examples
- Or also add a dedicated “canonical template contract” doc?

10. Strictness
- Should docs be phrased as:
  - recommended convention
- Or:
  - hard rule / canonical contract

## Observed Runtime Data Channels

1. Flowing payload
- The value that actually moves across edges is `NodeExecutionInput.payload`.
- Source:
  - `src/pipeline/nodes/interface.rs`
  - `src/pipeline/model.rs`

2. Metadata side-channel
- Every node also receives `NodeExecutionInput.metadata`.
- This is not the same thing as the upstream payload.
- Engine seeds it with:
  - `owner`
  - `project`
  - `pipeline`
  - `request_id`
  - `route`
  - `trigger`
  - `nodes` (snapshot of completed node outputs)

3. Config-expression scopes
- Before node execution, config `{{ expr }}` values are resolved with access to:
  - `$input`
  - `$trigger`
  - `$nodes`
  - `$ctx`
- This means node config can depend on more than the upstream payload alone.

4. Template/web render state
- For `n.web.response` / static web generation, the rendered template state starts from the payload,
  then trigger fields are injected from metadata:
  - `auth`
  - `params`
  - `query`
  - `headers`
- So template state is payload-plus-trigger-context, not just raw upstream payload.

5. Engine-level non-payload dependencies
- Nodes may also depend on engine-attached services that are not part of edge flow:
  - credentials
  - template root
  - data root
  - platform service
  - websocket hub
  - state bus
