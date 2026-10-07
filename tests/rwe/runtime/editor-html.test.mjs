// <DocumentHtml html>: stored document HTML shown through the document
// allowlist, with the real zeb/react runtime behind `zeb/react` and the
// shipped zeb/ui files loaded unchanged. Run from the repository root:
//
//   node --test tests/rwe/runtime/editor-html.test.mjs
//
// The page half (the compiler's SSR, and that a page using it compiles while
// raw HTML stays refused) is tests/rwe/zeb_ui_editor_html.rs; hydration in a
// browser is tests/e2e/specs/editor-html.spec.ts.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';

const ROOT = new URL('../../../', import.meta.url);
const UI = new URL('blessed/source-libraries/ui/0.1/src/', ROOT).href;
const REACT = new URL('src/rwe/runtime/zeb_react.mjs', ROOT).href;
register('data:text/javascript,' + encodeURIComponent(`
const UI = ${JSON.stringify(UI)};
export async function resolve(specifier, context, next) {
  if (specifier.startsWith('zeb/ui/')) return { url: UI + specifier.slice(7) + '.tsx', shortCircuit: true };
  if (specifier === 'zeb/react') return { url: ${JSON.stringify(REACT)}, shortCircuit: true };
  return next(specifier, context);
}
export async function load(url, context, next) {
  if (url.startsWith(UI) && url.endsWith('.tsx')) {
    const { readFile } = await import('node:fs/promises');
    return { format: 'module', source: await readFile(new URL(url), 'utf8'), shortCircuit: true };
  }
  return next(url, context);
}`));

const { h, renderToString } = await import('zeb/react');
const { DocumentHtml, parseDocumentHtml } = await import('zeb/ui/editor-html');
const { potoruExtension } = await import('zeb/ui/editor-potoru');
const { guardComponent, componentProps } = await import('zeb/ui/editor-component');

const WRAP = ['<div data-slot="document">', '</div>'];
/** What the page prints for `html`, without the wrapper. */
const shown = (html) => {
  const out = renderToString(h(DocumentHtml, { html }));
  assert.ok(out.startsWith(WRAP[0]) && out.endsWith(WRAP[1]), out);
  return out.slice(WRAP[0].length, -WRAP[1].length);
};

// Written as renderDocumentHtml writes (class last); the compiler's own
// round trip is in tests/rwe/zeb_ui_editor_html.rs.
test('HTML renderDocumentHtml wrote is shown byte for byte', () => {
  const stored = [
    '<h2 class="mt-6 mb-2 text-2xl font-semibold tracking-tight">Field notes</h2>',
    '<p class="my-1.5">Survey <strong>north</strong> &amp; <a href="https://example.com/a?b=1&amp;c=2" rel="noopener" class="text-info underline underline-offset-4">site-a</a>.</p>',
    '<figure data-figure="" class="my-4"><img src="/_files/uploads/demo.png" alt="A &quot;demo&quot; map" class="my-4 max-w-full rounded-lg"><figcaption class="mt-2 text-sm">Fig. 1</figcaption></figure>',
    '<ul data-todo="" class="my-1.5 list-none pl-0"><li data-checked="true" class="my-0.5 flex items-start gap-2"><input type="checkbox" checked="" disabled="" class="mt-2 size-4 shrink-0 accent-primary"><div class="min-w-0 flex-1"><p class="my-1.5">Done</p></div></li></ul>',
    '<pre data-language="ts" class="my-3"><code><span class="text-primary">const</span> x = 1 &lt; 2;</code></pre>',
    '<hr class="my-6 border-border"><p class="my-1.5">a<br>b</p>',
  ].join('');
  assert.equal(shown(stored), stored);
});

test('a Potoru block round-trips: the placeholder the page hydrates is kept exactly', () => {
  const ext = potoruExtension({ libraries: ['/_files/libs/basic.potolib'] });
  const block = renderToString(h(guardComponent(ext.component), componentProps({ type: 'potoru', attrs: {
    key: 'intro', src: '/_files/stories/intro.poto', mode: 'play', still: false, time: 0, libraries: [],
    snapshot: { title: 'Intro "one"', poster: '/_files/posters/intro.png' },
  } }, ext)));
  assert.match(block, /data-zeb-lib="potoru"/);
  assert.match(block, /data-config="\{&quot;src&quot;/);
  assert.equal(shown(block), block);
});

test('a tampered row cannot inject a script, handler, iframe, style or javascript: link', () => {
  const tampered = [
    '<p class="my-1.5" onclick="steal()" style="color:red">Hi<script>alert(1)</script></p>',
    '<img src="x" onerror="alert(1)" srcset="y 2x">',
    '<a href="javascript:alert(1)" target="_top">go</a>',
    '<a href=" JaVaScRiPt:alert(1)">case</a>',
    '<iframe src="https://example.com/x"></iframe>',
    '<style>body{display:none}</style>',
    '<svg><script>alert(1)</script></svg>',
    '<object data="x.swf"></object><embed src="x.swf">',
    '<form action="/x"><button>b</button></form>',
    '<div data-zeb-lib="potoru" data-config="{}" ONMOUSEOVER="x()"></div>',
    '<!-- <script>alert(1)</script> --><p>after</p>',
    '<textarea><script>alert(1)</script></textarea>',
    '<marquee>old</marquee>',
  ].join('');
  const out = shown(tampered);
  assert.equal(
    out,
    '<p class="my-1.5">Hi</p>' +
      '<img src="#">' +
      '<a href="#">go</a>' +
      '<a href="#">case</a>' +
      '<div data-zeb-lib="potoru" data-config="{}"></div>' +
      '<p>after</p>' +
      '<span>old</span>',
  );
  for (const bad of ['<script', 'onclick', 'onerror', 'onmouseover', 'style', 'javascript:', '<iframe', '<svg', '<object', '<embed', '<form', 'srcset', 'alert(']) {
    assert.ok(!out.toLowerCase().includes(bad), `${bad} reached the page: ${out}`);
  }
});

test('text is text: markup in an attribute or an entity never becomes an element', () => {
  assert.equal(shown('<p title="&lt;script&gt;">&lt;b&gt;x&lt;/b&gt; &#60;i&#62; &#x3C;u&#x3E;</p>'), '<p title="&lt;script&gt;">&lt;b&gt;x&lt;/b&gt; &lt;i&gt; &lt;u&gt;</p>');
  assert.equal(shown('a < b and <3 <'), 'a &lt; b and &lt;3 &lt;');
  assert.equal(shown(null), '');
  assert.equal(shown(42), '');
});

test('the same string builds the same tree every time (server and browser agree)', () => {
  const html = '<p class="my-1.5">one<em>two</em></p><div data-zeb-lib="potoru" data-config="{&quot;src&quot;:&quot;/a.poto&quot;}"></div>';
  assert.deepEqual(parseDocumentHtml(html), parseDocumentHtml(html));
  const once = shown(html);
  assert.equal(shown(once), once, 'showing the shown HTML changes nothing');
  assert.equal(renderToString(h(DocumentHtml, { html })), renderToString(h(DocumentHtml, { html })));
});

test('unclosed and stray tags stay inside the document', () => {
  assert.equal(shown('<p>open<strong>bold</p><p>next</p></div></div>'), '<p>open<strong>bold</strong></p><p>next</p>');
  const deep = '<div>'.repeat(500) + 'x' + '</div>'.repeat(500);
  assert.ok(shown(deep).includes('x'));
});
