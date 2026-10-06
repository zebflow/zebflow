// The composable editor's engine half: the real zeb/prosemirror bundle and
// the shipped zeb/ui extension files, loaded unchanged (with the real
// zeb/react runtime behind `zeb/react`). Run from the
// repository root:
//
//   node --test tests/rwe/runtime/editor.test.mjs
//
// The extension files are plain JavaScript inside .tsx (no JSX, no types) so
// the server, the browser and the engine load the same object; a hook below
// maps `zeb/ui/<name>` to the file and loads it as a module. A file that
// grows JSX fails here first.

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

const engine = await import(new URL('blessed/rwe-libraries/prosemirror/0.1/runtime/prosemirror.bundle.mjs', ROOT).href);
const { defineExtension } = await import('zeb/ui/editor-extension');
const { calloutExtension } = await import('zeb/ui/editor-callout');
const { tableExtension } = await import('zeb/ui/editor-table');
const { figureExtension } = await import('zeb/ui/editor-figure');
const { embedExtension } = await import('zeb/ui/editor-embed');
const { mentionExtension } = await import('zeb/ui/editor-mention');
const { citationExtension } = await import('zeb/ui/editor-citation');
const { pm, createSchema, adoptDocument, restoreDocument } = engine;

const EXTENSIONS = [
  calloutExtension(),
  tableExtension(),
  figureExtension(),
  embedExtension({ name: 'product', route: '/api/products/search' }),
  mentionExtension({ name: 'person', route: '/api/people/search' }),
  citationExtension({ route: '/api/works/search' }),
];
const schema = createSchema(undefined, EXTENSIONS);

// ProseMirror's attrs have a null prototype; compare what JSON would store.
const plain = (value) => JSON.parse(JSON.stringify(value));
const t = (text, marks) => (marks ? { type: 'text', text, marks } : { type: 'text', text });
// Every attr written out, as ProseMirror's toJSON writes it back.
const DOC = {
  type: 'doc',
  content: [
    { type: 'heading', attrs: { level: 2 }, content: [t('Notes')] },
    { type: 'paragraph', content: [
      t('Met '),
      { type: 'person', attrs: { id: 'p1', label: 'Alex Example', href: '/people/p1', snapshot: { description: 'Maps' } } },
      t(' citing '),
      { type: 'citation', attrs: { id: '10.5555/a', label: 'Example 2020', href: null, snapshot: { title: 'On samples', year: 2020 } } },
      t(' here', [{ type: 'link', attrs: { href: 'https://example.com', title: null } }]),
    ] },
    { type: 'callout', attrs: { icon: '!', tone: 'info' }, content: [{ type: 'paragraph', content: [t('Inside')] }] },
    { type: 'table', content: [
      { type: 'table_row', content: [{ type: 'table_header', content: [t('A')] }, { type: 'table_header', content: [t('B')] }] },
      { type: 'table_row', content: [{ type: 'table_cell', content: [t('1')] }, { type: 'table_cell', content: [t('2')] }] },
    ] },
    { type: 'figure', attrs: { src: '/files/x.png', alt: 'x', ref: 'uploads/x.png', credit: 'Photo: Example' }, content: [t('Caption')] },
    { type: 'product', attrs: { key: 'notebook', snapshot: { title: 'Field notebook', href: '/shop/notebook' } } },
  ],
};

test('extensions compose into one schema, after the engine\'s own nodes', () => {
  const names = Object.keys(schema.nodes);
  for (const type of ['callout', 'table', 'table_row', 'table_header', 'table_cell', 'figure', 'product', 'person', 'citation']) {
    assert.ok(names.includes(type), type);
  }
  assert.ok(names.indexOf('paragraph') < names.indexOf('callout'), 'paragraph stays the default block');
  assert.equal(schema.nodes.person.isInline, true);
  assert.equal(schema.nodes.person.isAtom, true);
  assert.equal(schema.nodes.figure.inlineContent, true);
  assert.equal(schema.nodes.product.isBlock, true);
  assert.equal(schema.nodes.table.contentMatch.defaultType, schema.nodes.table_row);
  // No extension, no callout: it is not one of the engine's own blocks any more.
  assert.equal(createSchema().nodes.callout, undefined);
});

test('a type defined twice, or over the engine\'s own, is refused by name', () => {
  assert.throws(() => createSchema(undefined, [calloutExtension(), calloutExtension()]), /editor extension "callout" redefines "callout"/);
  const mine = defineExtension({ name: 'mine', nodes: { heading: { spec: { group: 'block', content: 'inline*' } } } });
  assert.throws(() => createSchema(undefined, [mine]), /redefines "heading"/);
});

test('a document round-trips through the engine unchanged', () => {
  const node = pm.Node.fromJSON(schema, adoptDocument(schema, DOC));
  node.check();
  assert.deepEqual(plain(restoreDocument(node.toJSON())), DOC);
});

test('a node or mark the schema lacks survives load and save', () => {
  const doc = {
    type: 'doc',
    content: [
      { type: 'paragraph', content: [t('a '), { type: 'emoji', attrs: { name: 'wave' } }, t('b', [{ type: 'glow', attrs: { level: 2 } }])] },
      { type: 'pullquote', attrs: { by: 'x' }, content: [t('kept text')] },
      { type: 'gallery', attrs: { n: 2 }, content: [{ type: 'paragraph', content: [t('inside')] }] },
      { type: 'figure', attrs: { src: '', alt: '', ref: null, credit: '' }, content: [t('needs the figure extension')] },
    ],
  };
  // The base schema knows none of emoji, glow, pullquote, gallery or figure.
  const base = createSchema();
  const adopted = adoptDocument(base, doc);
  const node = pm.Node.fromJSON(base, adopted);
  node.check();
  assert.deepEqual(plain(restoreDocument(node.toJSON())), doc);
  const types = [];
  node.descendants((child) => { types.push(child.type.name); });
  assert.ok(types.includes('unknown_inline') && types.includes('unknown_text') && types.includes('unknown_block'), types.join());
});

test('the editor draws an extension through the same render, sanitized, with a content hole', () => {
  const doc = pm.Node.fromJSON(schema, DOC);
  const figure = doc.child(4);
  const spec = schema.nodes.figure.spec.toDOM(figure);
  assert.equal(spec[0], 'figure');
  const caption = spec.find((part) => Array.isArray(part) && part[0] === 'figcaption');
  assert.deepEqual(caption.slice(2), [0], 'the caption is the content hole, alone in its element');
  const img = spec.find((part) => Array.isArray(part) && part[0] === 'img');
  assert.equal(img[1].contenteditable, 'false');

  const person = schema.nodes.person.spec.toDOM(doc.child(1).child(1));
  assert.deepEqual(person, ['a', { class: 'rounded bg-accent px-1 font-medium text-accent-foreground no-underline', 'data-mention': 'person', 'data-id': 'p1', href: '/people/p1' }, '@Alex Example']);
  // In the editor a citation shows its label; numbers belong to the rendered page.
  const citation = schema.nodes.citation.spec.toDOM(doc.child(1).child(3));
  assert.equal(citation[2], '[Example 2020]');

  const evil = defineExtension({
    name: 'evil', node: { group: 'block', content: 'inline*' },
    render: (_n, r) => r.h('div', { onclick: 'x()', style: 'color:red', class: 'a', contenteditable: 'true' }, r.h('script', {}, 'x()'), r.h('a', { href: 'javascript:x()' }, r.content)),
  });
  const out = evil.nodes.evil.toDOM({ attrs: {} });
  assert.deepEqual(out, ['div', { class: 'a' }, ['a', { href: '#' }, 0]]);
});

test('table commands move between cells and add and remove rows and columns', () => {
  const doc = pm.Node.fromJSON(schema, { type: 'doc', content: [DOC.content[3]] });
  let state = pm.EditorState.create({ doc, selection: pm.TextSelection.create(doc, 4) });
  const cmd = EXTENSIONS[1].commands(schema, pm);
  const run = (command) => { let next = null; assert.ok(command(state, (tr) => { next = state.apply(tr); })); state = next; };
  const cellText = () => state.selection.$from.parent.textContent;
  assert.equal(cellText(), 'A');
  run(cmd.tableNextCell);
  assert.equal(cellText(), 'B');
  run(cmd.tableCellBelow);
  assert.equal(cellText(), '2');
  run(cmd.tableNextCell); // past the last cell: a new row
  assert.equal(state.doc.child(0).childCount, 3);
  run(cmd.tableAddColumn);
  assert.deepEqual(Array.from({ length: 3 }, (_, r) => state.doc.child(0).child(r).childCount), [3, 3, 3]);
  assert.equal(state.doc.child(0).child(0).child(1).type.name, 'table_header', 'a column added to the header row is a header');
  run(cmd.tableDeleteColumn);
  run(cmd.tableDeleteRow);
  assert.equal(state.doc.child(0).childCount, 2);
  assert.deepEqual([state.doc.child(0).child(0).childCount, state.doc.child(0).child(1).childCount], [2, 2]);
  state.doc.check();
});

test('insertTable builds a header row and plain rows', () => {
  const doc = pm.Node.fromJSON(schema, { type: 'doc', content: [{ type: 'paragraph' }] });
  let state = pm.EditorState.create({ doc });
  const cmd = EXTENSIONS[1].commands(schema, pm);
  cmd.insertTable(3, 2)(state, (tr) => { state = state.apply(tr); });
  const table = state.doc.firstChild;
  assert.equal(table.type.name, 'table');
  assert.deepEqual(table.content.content.map((row) => row.content.content.map((cell) => cell.type.name)), [
    ['table_header', 'table_header'], ['table_cell', 'table_cell'], ['table_cell', 'table_cell'],
  ]);
});

const { referenceExtension } = await import('zeb/ui/editor-reference');
const { potoruExtension, potoruConfig } = await import('zeb/ui/editor-potoru');

// What a project configures: two mention kinds, a reference link and card, a story block.
const SOURCES = [
  mentionExtension({ name: 'person', trigger: '@', route: '/api/people/search' }),
  mentionExtension({ name: 'org', trigger: '+', route: '/api/orgs/search' }),
  referenceExtension({ name: 'article', route: '/api/articles/search' }),
  referenceExtension({ name: 'page_card', variant: 'card', route: '/api/pages/search' }),
  potoruExtension(),
];
const sourceSchema = createSchema(undefined, SOURCES);
const SOURCE_DOC = {
  type: 'doc',
  content: [
    { type: 'paragraph', content: [
      { type: 'person', attrs: { id: 'p1', label: 'Alex Example', href: '/people/p1', snapshot: null } },
      t(' of '),
      { type: 'org', attrs: { id: 'o1', label: 'Example Lab', href: '/orgs/o1', snapshot: { city: 'Sampleton' } } },
      t(' wrote '),
      { type: 'article', attrs: { key: 'a-1', label: 'Field guide', href: '/articles/field-guide', snapshot: { description: 'How we survey' } } },
    ] },
    { type: 'page_card', attrs: { key: 'pg-1', label: 'Site map', href: '/pages/map', snapshot: { meta: 'Weekly' } } },
    { type: 'potoru', attrs: { key: 'intro', src: '/_files/stories/intro.poto', mode: 'play', still: true, time: 2, libraries: [], snapshot: { title: 'Intro', poster: '' } } },
  ],
};

test('mention kinds, references and the story block compose and round-trip', () => {
  assert.equal(sourceSchema.nodes.org.isInline, true);
  assert.equal(sourceSchema.nodes.article.isInline, true);
  assert.equal(sourceSchema.nodes.page_card.isBlock, true);
  assert.equal(sourceSchema.nodes.potoru.isAtom && sourceSchema.nodes.potoru.isBlock, true);
  // Each kind owns one trigger; the engine's trigger plugin reads them from the list.
  assert.deepEqual(SOURCES.filter((ext) => ext.trigger).map((ext) => [ext.name, ext.trigger]), [['person', '@'], ['org', '+']]);
  const node = pm.Node.fromJSON(sourceSchema, adoptDocument(sourceSchema, SOURCE_DOC));
  node.check();
  assert.deepEqual(plain(restoreDocument(node.toJSON())), SOURCE_DOC);
});

const { h, renderToString } = await import('zeb/react');
const { guardComponent, componentProps, withNodeViews } = await import('zeb/ui/editor-component');
const { PotoruBlock } = await import('zeb/ui/editor-potoru');
const { potoruLibraryUrl, mergePotoruLibraries } = await import('zeb/ui/editor-potoru-libraries');

/** A story block as the page prints it: its component, through the guard. */
const printBlock = (ext, attrs) => renderToString(h(guardComponent(ext.component), componentProps({ type: ext.name, attrs }, ext)));

test('a picked item becomes key + snapshot; the story block draws PotoPlayer\'s placeholder', () => {
  const item = { id: 42, label: 'Field guide', href: '/articles/field-guide', snapshot: { description: 'How we survey' } };
  assert.deepEqual(SOURCES[2].picker.toAttrs(item), { key: '42', label: 'Field guide', href: '/articles/field-guide', snapshot: { description: 'How we survey' } });
  const story = potoruExtension({ search: async () => [] });
  assert.deepEqual(story.picker.toAttrs({ id: 'intro', label: 'Intro', href: '/_files/stories/intro.poto', snapshot: { poster: '/_files/stories/intro.png', libraries: ['/_files/libs/a.potolib', 'http://example.com/b.potolib'] } }),
    { key: 'intro', src: '/_files/stories/intro.poto', mode: '', still: false, time: 0, libraries: ['/_files/libs/a.potolib'], snapshot: { title: 'Intro', poster: '/_files/stories/intro.png' } });
  assert.equal(potoruExtension().picker, null, 'without a source the address is pasted in the panel');

  const html = printBlock(SOURCES[4], SOURCE_DOC.content[2].attrs);
  assert.equal(html, '<figure data-potoru-block="potoru" data-key="intro" class="my-4"><div data-zeb-lib="potoru" data-zeb-wrapper="PotoPlayer" data-config="{&quot;src&quot;:&quot;/_files/stories/intro.poto&quot;,&quot;controls&quot;:true,&quot;still&quot;:true,&quot;time&quot;:2,&quot;mode&quot;:&quot;play&quot;}" class="relative block w-full overflow-hidden rounded-lg bg-muted"></div><figcaption class="mt-2 text-sm text-muted-foreground">Intro</figcaption></figure>');
  // The panel writes strings and booleans; the config stays PotoPlayer's.
  assert.deepEqual(potoruConfig({ src: 'vbscript:x', still: 'true', time: '3', mode: 'nope' }), { src: '#', controls: true, still: true, time: 3 });
  assert.deepEqual(potoruConfig({ src: '/a.poto', still: false, time: 3 }), { src: '/a.poto', controls: true });
  // The page shows nothing for an empty block; the editor shows where to put an address.
  assert.equal(printBlock(SOURCES[4], { src: '' }), '');
  const editing = renderToString(h(SOURCES[4].editComponent, componentProps({ type: 'potoru', attrs: { src: '' } }, SOURCES[4], { edit: true, readOnly: false })));
  assert.match(editing, /set its \.poto address in the panel/);
  assert.match(editing, /data-slot="potoru-libraries"/);
});

test('ProseMirror serializes a component node as its attrs and reads it back (copy and paste)', () => {
  const doc = pm.Node.fromJSON(sourceSchema, SOURCE_DOC);
  const block = doc.child(2);
  const spec = sourceSchema.nodes.potoru.spec.toDOM(block);
  assert.equal(spec[0], 'div');
  assert.equal(spec[1]['data-zeb-node'], 'potoru');
  assert.deepEqual(JSON.parse(spec[1]['data-attrs']), plain(block.attrs));
  const rule = sourceSchema.nodes.potoru.spec.parseDOM[0];
  assert.equal(rule.tag, 'div[data-zeb-node="potoru"]');
  assert.deepEqual(rule.getAttrs({ getAttribute: () => spec[1]['data-attrs'] }), plain(block.attrs));
  assert.equal(rule.getAttrs({ getAttribute: () => '{not json' }), false);
});

test('story libraries: https or a path on this site; defaults merge with the block\'s own, the block wins a file name', () => {
  for (const ok of ['https://example.com/libs/basic.potolib', '/_files/libs/basic.potolib', '  /libs/x.potolib  ']) assert.ok(potoruLibraryUrl(ok), ok);
  for (const bad of ['http://example.com/a.potolib', 'javascript:alert(1)', '//example.com/a.potolib', 'libs/a.potolib', '/a b.potolib', '/a".potolib', 'https://', '', null, 42]) {
    assert.equal(potoruLibraryUrl(bad), null, String(bad));
  }
  const defaults = ['https://example.com/libs/basic.potolib', '/_files/libs/shapes.potolib'];
  assert.deepEqual(mergePotoruLibraries(defaults, ['/_files/libs/basic.potolib', '/_files/libs/extra.potolib', 'http://example.com/nope.potolib', '/_files/libs/extra.potolib']),
    ['/_files/libs/shapes.potolib', '/_files/libs/basic.potolib', '/_files/libs/extra.potolib']);
  assert.deepEqual(mergePotoruLibraries(defaults, undefined), defaults);

  assert.throws(() => potoruExtension({ libraries: ['http://example.com/a.potolib'] }), /library "http:\/\/example.com\/a.potolib" must be an https:\/\/ address or a path on this site/);
  assert.throws(() => potoruExtension({ libraries: 'https://example.com/a.potolib' }), /libraries is a list/);
  const story = potoruExtension({ libraries: defaults });
  assert.deepEqual(story.options.libraries, defaults);
  const html = printBlock(story, { key: 's', src: '/s.poto', libraries: ['/_files/libs/own.potolib', 'javascript:x'] });
  const config = JSON.parse(html.match(/data-config="([^"]*)"/)[1].replace(/&quot;/g, '"'));
  assert.deepEqual(config, { src: '/s.poto', libraries: [...defaults, '/_files/libs/own.potolib'], controls: true });
});

test('a node drawn by a component: the contract is checked, its props are the same everywhere, its output is sanitized', () => {
  const Badge = ({ attrs, options }) => h('span', { className: 'badge', 'data-tone': options.tone }, attrs.label);
  const badge = defineExtension({ name: 'badge', node: { inline: true, group: 'inline', atom: true, attrs: { label: { default: '' } } }, component: Badge, options: { tone: 'info' } });
  assert.deepEqual(badge.options, { tone: 'info' });
  const props = componentProps({ type: 'badge', attrs: { label: 'New' } }, badge);
  assert.deepEqual(Object.keys(props), ['node', 'attrs', 'options', 'edit', 'selected', 'readOnly', 'update']);
  assert.deepEqual([props.node, props.edit, props.selected, props.readOnly, typeof props.update], [{ type: 'badge', attrs: { label: 'New' } }, false, false, true, 'function']);
  assert.equal(renderToString(h(guardComponent(Badge), props)), '<span data-tone="info" class="badge">New</span>');

  const refused = (def) => { try { defineExtension(def); return 'ok'; } catch (err) { return err.message; } };
  const atom = { group: 'block', atom: true, attrs: {} };
  assert.match(refused({ name: 'x', node: atom, component: Badge, render: () => null }), /has both render and component/);
  assert.match(refused({ name: 'x', node: { group: 'block', content: 'inline*' }, component: Badge }), /is an atom: give it attrs, not content/);
  assert.match(refused({ name: 'x', mark: {}, component: Badge }), /a mark is drawn by render, not by a component/);
  assert.match(refused({ name: 'x', node: atom, editComponent: 'Badge' }), /editComponent of "x" must be a component/);
  assert.equal(refused({ name: 'x', node: atom, editComponent: Badge, render: (n, r) => r.h('div', {}) }), 'ok', 'an editor view with a declared static render');

  // Whatever a stored document says, and whatever tags a component writes, the page gets the allowlist.
  const Hostile = ({ attrs }) => h('div', { className: 'card', style: { color: 'red' }, onclick: 'alert(1)', onClick: () => {}, dangerouslySetInnerHTML: { __html: '<b>x</b>' } },
    h('a', { href: attrs.href }, 'open'), h('script', null, 'alert(1)'), h('img', { src: attrs.href, srcset: 'x 1x' }), h(Inner, { text: attrs.label }), [h('iframe', { src: '/x' })]);
  const Inner = ({ text }) => h('em', { onmouseover: 'x' }, text, h('style', null, 'body{}'));
  const out = renderToString(h(guardComponent(Hostile), componentProps({ type: 'x', attrs: { href: 'javascript:alert(1)', label: '<b>' } }, null)));
  assert.equal(out, '<div class="card"><a href="#">open</a><img src="#"><em>&lt;b&gt;</em></div>');
  assert.equal(guardComponent(Hostile), guardComponent(Hostile), 'one wrapper per component, so hydration sees the same type');
});

test('the editor gets a node view for each component node, through a plugin; other extensions pass unchanged', () => {
  const callout = calloutExtension();
  assert.equal(withNodeViews(callout, () => null, false), callout);
  const story = potoruExtension();
  const viewed = withNodeViews(story, () => null, false);
  assert.notEqual(viewed, story);
  assert.equal(viewed.name, 'potoru');
  const plugins = viewed.plugins(sourceSchema, pm);
  assert.equal(plugins.length, 1);
  assert.deepEqual(Object.keys(plugins[0].props.nodeViews), ['potoru']);
});
