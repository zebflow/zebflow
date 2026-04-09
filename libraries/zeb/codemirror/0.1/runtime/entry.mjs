import {
  CompletionContext,
  EditorView,
  autocompletion,
  basicSetup,
  css,
  cssLanguage,
  javascript,
  javascriptLanguage,
  lintGutter,
  linter,
  oneDark,
  setDiagnostics,
  snippetCompletion,
} from "./codemirror.bundle.mjs";

const JS_LIKE_KINDS = new Set([
  "template",
  "tsx",
  "typescript",
  "ts",
  "javascript",
  "js",
  "jsx",
]);

const DEFAULT_IMPORT_SOURCES = [
  "zeb",
  "zeb/use",
  "zeb/deckgl",
  "zeb/markdown",
  "zeb/pdf",
  "zeb/d3",
  "zeb/icons",
  "zeb/codemirror",
  "zeb/graphui",
  "zeb/preact",
  "zeb/prosemirror",
  "zeb/threejs",
  "zeb/threejs-vrm",
];

const TOOL_NAMESPACE_OPTIONS = [
  {
    label: "time",
    type: "namespace",
    detail: "Tool.time",
    info: "Date/time helpers: format, diff, add, relativeTime, tz, Hijri conversion.",
  },
  {
    label: "arr",
    type: "namespace",
    detail: "Tool.arr",
    info: "Array/data shaping helpers: sortBy, filterBy, paginate, groupBy, sumBy, uniqueBy.",
  },
  {
    label: "stat",
    type: "namespace",
    detail: "Tool.stat",
    info: "Statistics helpers: mean, median, variance, percentile, correlation, linreg, histogram.",
  },
  {
    label: "geo",
    type: "namespace",
    detail: "Tool.geo",
    info: "Geospatial helpers: distance, bbox, center, pointInPolygon, centroid, nearestPoint.",
  },
];

const TOOL_MEMBER_OPTIONS = {
  time: [
    fnOption("format", "Tool.time.format(date, pattern, locale?)", "Format a date using Zebflow tokens."),
    fnOption("diff", "Tool.time.diff(a, b, unit?)", "Return difference between two dates."),
    fnOption("add", "Tool.time.add(date, amount, unit)", "Add time units to a date."),
    fnOption("subtract", "Tool.time.subtract(date, amount, unit)", "Subtract time units from a date."),
    fnOption("startOf", "Tool.time.startOf(date, unit)", "Snap to the start of day/week/month/year."),
    fnOption("endOf", "Tool.time.endOf(date, unit)", "Snap to the end of day/month/year."),
    fnOption("isBefore", "Tool.time.isBefore(a, b)", "True when a is before b."),
    fnOption("isAfter", "Tool.time.isAfter(a, b)", "True when a is after b."),
    fnOption("isSame", "Tool.time.isSame(a, b, unit?)", "Compare dates, optionally at a given unit."),
    fnOption("relativeTime", "Tool.time.relativeTime(date, locale?)", "Human-readable relative time string."),
    fnOption("tz", "Tool.time.tz(date, timezone)", "Convert a date to an IANA timezone."),
    fnOption("toHijri", "Tool.time.toHijri(date)", "Convert Gregorian date to Hijri parts."),
    fnOption("fromHijri", "Tool.time.fromHijri(day, month, year)", "Convert Hijri date to Date."),
    fnOption("locale", "Tool.time.locale(code)", "Set the default locale for Tool.time."),
  ],
  arr: [
    fnOption("sortBy", "Tool.arr.sortBy(data, key, dir?)", "Sort array items by property or selector."),
    fnOption("filterBy", "Tool.arr.filterBy(data, filters)", "Filter an array by object, text, or predicate."),
    fnOption("paginate", "Tool.arr.paginate(data, page, size)", "Return paginated items and totals."),
    fnOption("groupBy", "Tool.arr.groupBy(data, key)", "Group items by property or selector."),
    fnOption("flatGroupBy", "Tool.arr.flatGroupBy(data, key)", "Return grouped items as flat objects."),
    fnOption("sumBy", "Tool.arr.sumBy(data, key)", "Sum a numeric field across items."),
    fnOption("countBy", "Tool.arr.countBy(data, key)", "Count items by group."),
    fnOption("uniqueBy", "Tool.arr.uniqueBy(data, key)", "Keep only the first item for each key."),
  ],
  stat: [
    fnOption("mean", "Tool.stat.mean(values)", "Average of numeric values."),
    fnOption("median", "Tool.stat.median(values)", "Median of numeric values."),
    fnOption("variance", "Tool.stat.variance(values)", "Population variance."),
    fnOption("stddev", "Tool.stat.stddev(values)", "Standard deviation."),
    fnOption("percentile", "Tool.stat.percentile(values, p)", "Percentile at p (0-100)."),
    fnOption("zscore", "Tool.stat.zscore(values)", "Z-scores for each value."),
    fnOption("rateAbove", "Tool.stat.rateAbove(values, threshold)", "Percent of values above threshold."),
    fnOption("correlation", "Tool.stat.correlation(xs, ys)", "Pearson correlation."),
    fnOption("linreg", "Tool.stat.linreg(xs, ys)", "Linear regression slope/intercept/r2."),
    fnOption("histogram", "Tool.stat.histogram(values, bins)", "Histogram bins with counts."),
  ],
  geo: [
    fnOption("distance", "Tool.geo.distance(from, to)", "Distance in km between [lon, lat] points."),
    fnOption("bbox", "Tool.geo.bbox(features)", "Bounding box [minLon, minLat, maxLon, maxLat]."),
    fnOption("center", "Tool.geo.center(pointsOrFeatures)", "Center point of a bbox."),
    fnOption("pointInPolygon", "Tool.geo.pointInPolygon(point, polygon)", "True when point is inside Polygon or MultiPolygon."),
    fnOption("centroid", "Tool.geo.centroid(geometry)", "Centroid of Polygon or MultiPolygon."),
    fnOption("nearestPoint", "Tool.geo.nearestPoint(origin, points)", "Nearest point index and distance in km."),
  ],
};

function normalizeLanguageKind(kind) {
  return String(kind || "").trim().toLowerCase();
}

function resolveLanguageExtension(kind) {
  switch (normalizeLanguageKind(kind)) {
    case "template":
    case "tsx":
    case "typescript":
    case "ts":
      return javascript({ jsx: true, typescript: true });
    case "javascript":
    case "js":
    case "jsx":
      return javascript({ jsx: true, typescript: false });
    case "css":
    case "style":
      return css();
    default:
      return null;
  }
}

function buildEditorTheme(options = {}) {
  const root = {};
  const scroller = {};

  if (options.height) {
    root.height = options.height;
    scroller.height = "100%";
  }
  if (options.minHeight) {
    root.minHeight = options.minHeight;
  }
  if (options.maxHeight) {
    root.maxHeight = options.maxHeight;
    scroller.maxHeight = options.maxHeight;
  }
  if (options.scrollerOverflow) {
    scroller.overflow = options.scrollerOverflow;
  } else if (options.height || options.maxHeight) {
    scroller.overflow = "auto";
  }

  if (!Object.keys(root).length && !Object.keys(scroller).length) {
    return null;
  }

  const spec = {};
  if (Object.keys(root).length) {
    spec["&"] = root;
  }
  if (Object.keys(scroller).length) {
    spec[".cm-scroller"] = scroller;
  }
  return EditorView.theme(spec);
}

function fnOption(label, detail, info) {
  return {
    label,
    type: "function",
    detail,
    info,
  };
}

function isJavaScriptLikeKind(kind) {
  return JS_LIKE_KINDS.has(normalizeLanguageKind(kind));
}

function normalizeProjectImportSpecifier(relPath) {
  if (typeof relPath !== "string") {
    return null;
  }

  let normalized = relPath.trim().replace(/^\/+/, "");
  if (!normalized || normalized.startsWith("docs/")) {
    return null;
  }
  if (normalized.endsWith(".zf.json")) {
    return null;
  }

  if (/\.(tsx|ts|jsx|js|mjs)$/i.test(normalized)) {
    normalized = normalized.replace(/\.(tsx|ts|jsx|js|mjs)$/i, "");
  } else if (!/\.(css|json|svg|png|jpg|jpeg|gif|webp|avif)$/i.test(normalized)) {
    return null;
  }
  return `@/${normalized}`;
}

function uniqueSorted(values) {
  return Array.from(new Set(values.filter(Boolean))).sort((a, b) => a.localeCompare(b));
}

function collectImportSpecifiers(options = {}) {
  const specifiers = [...DEFAULT_IMPORT_SOURCES];
  const projectFiles = Array.isArray(options.projectFiles) ? options.projectFiles : [];
  const importSources = Array.isArray(options.importSources) ? options.importSources : [];

  for (const relPath of projectFiles) {
    specifiers.push(normalizeProjectImportSpecifier(relPath));
  }
  for (const source of importSources) {
    if (typeof source === "string" && source.trim()) {
      specifiers.push(source.trim());
    }
  }

  return uniqueSorted(specifiers);
}

function createProjectImportIndex(projectFiles) {
  const exact = new Map();
  const normalized = uniqueSorted(Array.isArray(projectFiles) ? projectFiles : [])
    .map((value) => String(value || "").replace(/^\/+/, ""))
    .filter(Boolean);

  for (const relPath of normalized) {
    exact.set(relPath, relPath);
  }

  return { exact, files: normalized };
}

function resolveProjectImportPath(specifier, projectImportIndex) {
  if (!specifier || !specifier.startsWith("@/") || !projectImportIndex) {
    return null;
  }

  const base = specifier.slice(2).replace(/^\/+/, "");
  if (!base) {
    return null;
  }

  const candidates = [
    base,
    `${base}.tsx`,
    `${base}.ts`,
    `${base}.jsx`,
    `${base}.js`,
    `${base}.mjs`,
    `${base}.css`,
    `${base}.json`,
    `${base}.svg`,
    `${base}.png`,
    `${base}.jpg`,
    `${base}.jpeg`,
    `${base}.gif`,
    `${base}.webp`,
    `${base}.avif`,
    `${base}/index.tsx`,
    `${base}/index.ts`,
    `${base}/index.jsx`,
    `${base}/index.js`,
    `${base}/index.mjs`,
  ];

  for (const candidate of candidates) {
    const found = projectImportIndex.exact.get(candidate);
    if (found) {
      return found;
    }
  }

  return null;
}

function resolveImportTarget(specifier, options = {}, projectImportIndex) {
  const projectPath = resolveProjectImportPath(specifier, projectImportIndex);
  if (projectPath) {
    return {
      kind: "project",
      specifier,
      relPath: projectPath,
    };
  }

  if (specifier === "zeb" || specifier.startsWith("zeb/")) {
    return {
      kind: "library",
      specifier,
    };
  }

  return null;
}

function createOutlineLoader(options = {}) {
  const cache = new Map();
  const endpoint =
    typeof options.templateOutlineUrl === "string" && options.templateOutlineUrl.trim()
      ? new URL(options.templateOutlineUrl, document.baseURI).href
      : null;

  if (!endpoint) {
    return null;
  }

  return async (relPath) => {
    const normalized = String(relPath || "").replace(/^\/+/, "");
    if (!normalized) {
      return null;
    }
    if (cache.has(normalized)) {
      return cache.get(normalized);
    }

    const promise = (async () => {
      try {
        const response = await fetch(`${endpoint}?path=${encodeURIComponent(normalized)}`, {
          headers: { Accept: "application/json" },
        });
        const payload = await response.json().catch(() => null);
        if (!response.ok) {
          return null;
        }
        return payload?.outline || null;
      } catch (_error) {
        return null;
      }
    })();

    cache.set(normalized, promise);
    return promise;
  };
}

function parseImportStatement(lineText) {
  if (typeof lineText !== "string" || !lineText.includes("from")) {
    return null;
  }

  const match = lineText.match(/^\s*(?:import|export)\s+(.+?)\s+from\s+(['"])([^'"]+)\2/);
  if (!match) {
    return null;
  }

  const clause = match[1];
  const specifier = match[3];
  const full = match[0];
  const start = match.index || 0;
  const clauseStart = start + full.indexOf(clause);
  const specStart = lineText.indexOf(specifier, clauseStart + clause.length);
  const specEnd = specStart + specifier.length;
  const refs = [];

  const braceOpen = clause.indexOf("{");
  const braceClose = braceOpen >= 0 ? clause.indexOf("}", braceOpen + 1) : -1;

  const namedFrom = braceOpen >= 0 && braceClose > braceOpen ? clauseStart + braceOpen + 1 : null;
  const namedTo = braceOpen >= 0 && braceClose > braceOpen ? clauseStart + braceClose : null;

  const prefix = braceOpen >= 0 ? clause.slice(0, braceOpen).trim().replace(/,$/, "").trim() : clause.trim();

  if (prefix) {
    const namespaceMatch = prefix.match(/\*\s+as\s+([A-Za-z_$][\w$]*)/);
    if (namespaceMatch) {
      const localName = namespaceMatch[1];
      const localIndex = clause.indexOf(localName);
      refs.push({
        mode: "namespace",
        localName,
        exportName: "*",
        from: clauseStart + localIndex,
        to: clauseStart + localIndex + localName.length,
      });
    } else {
      const defaultName = prefix.split(",")[0].trim();
      if (defaultName && /^[A-Za-z_$][\w$]*$/.test(defaultName)) {
        const defaultIndex = clause.indexOf(defaultName);
        refs.push({
          mode: "default",
          localName: defaultName,
          exportName: "default",
          from: clauseStart + defaultIndex,
          to: clauseStart + defaultIndex + defaultName.length,
        });
      }
    }
  }

  if (namedFrom != null && namedTo != null) {
    const namedClause = clause.slice(braceOpen + 1, braceClose);
    const matcher = /([A-Za-z_$][\w$]*)(\s+as\s+([A-Za-z_$][\w$]*))?/g;
    let item;
    while ((item = matcher.exec(namedClause))) {
      const exportName = item[1];
      const localName = item[3] || exportName;
      const localOffset = item[3] ? item[0].lastIndexOf(localName) : item.index;
      refs.push({
        mode: "named",
        localName,
        exportName,
        from: namedFrom + localOffset,
        to: namedFrom + localOffset + localName.length,
      });
    }
  }

  return {
    clause,
    clauseStart,
    clauseEnd: clauseStart + clause.length,
    specifier,
    specStart,
    specEnd,
    refs,
    namedFrom,
    namedTo,
  };
}

function extractImportSpecifierPrefix(before) {
  const patterns = [
    /\bfrom\s+["']([^"']*)$/,
    /\bimport\s*\(\s*["']([^"']*)$/,
    /\bimport\s+["']([^"']*)$/,
    /\bexport\s+[^"']*from\s+["']([^"']*)$/,
  ];

  for (const pattern of patterns) {
    const match = before.match(pattern);
    if (match) {
      return match[1] || "";
    }
  }

  return null;
}

function buildImportCompletionOptions(prefix, specifiers) {
  if (prefix == null) {
    return [];
  }

  if (prefix === "") {
    return [
      { label: "@/", type: "namespace", detail: "Project templates" },
      ...specifiers
        .filter((value) => value === "zeb" || value.startsWith("zeb/"))
        .map((value) => ({
          label: value,
          type: value.includes("/") ? "module" : "namespace",
          detail: "Zeb library",
        })),
    ];
  }

  const options = new Map();
  for (const specifier of specifiers) {
    if (!specifier.startsWith(prefix)) {
      continue;
    }
    if (specifier === prefix) {
      options.set(specifier, {
        label: specifier,
        type: "module",
        detail: specifier.startsWith("@/") ? "Project import" : "Library import",
      });
      continue;
    }

    const rest = specifier.slice(prefix.length);
    const boundary = rest.startsWith("/") ? "/" : "";
    const remainder = boundary ? rest.slice(1) : rest;
    if (!remainder) {
      continue;
    }

    const slashIndex = remainder.indexOf("/");
    if (slashIndex >= 0) {
      const folderLabel = prefix + boundary + remainder.slice(0, slashIndex + 1);
      if (!options.has(folderLabel)) {
        options.set(folderLabel, {
          label: folderLabel,
          type: "namespace",
          detail: "Import folder",
        });
      }
      continue;
    }

    options.set(specifier, {
      label: specifier,
      type: "module",
      detail: specifier.startsWith("@/") ? "Project import" : "Library import",
    });
  }

  return Array.from(options.values()).sort((a, b) => a.label.localeCompare(b.label));
}

function findImportReferenceAt(state, pos) {
  const line = state.doc.lineAt(pos);
  const parsed = parseImportStatement(line.text);
  if (!parsed) {
    return null;
  }

  const offset = pos - line.from;
  if (offset >= parsed.specStart && offset <= parsed.specEnd) {
    return {
      kind: "source",
      specifier: parsed.specifier,
      from: line.from + parsed.specStart,
      to: line.from + parsed.specEnd,
    };
  }

  for (const ref of parsed.refs) {
    if (offset >= ref.from && offset <= ref.to) {
      return {
        kind: "symbol",
        specifier: parsed.specifier,
        symbolName: ref.exportName,
        localName: ref.localName,
        isDefault: ref.mode === "default",
        from: line.from + ref.from,
        to: line.from + ref.to,
      };
    }
  }

  return null;
}

function findNamedImportContext(state, pos) {
  const line = state.doc.lineAt(pos);
  const parsed = parseImportStatement(line.text);
  if (!parsed || parsed.namedFrom == null || parsed.namedTo == null) {
    return null;
  }

  const offset = pos - line.from;
  if (offset < parsed.namedFrom || offset > parsed.namedTo) {
    return null;
  }

  const typedPrefix = line.text.slice(parsed.namedFrom, offset);
  const partialMatch = typedPrefix.match(/([A-Za-z_$][\w$]*)$/);
  const partial = partialMatch ? partialMatch[1] : "";
  const importedNames = parsed.refs
    .filter((ref) => ref.mode === "named")
    .map((ref) => ref.exportName);

  return {
    specifier: parsed.specifier,
    partial,
    from: pos - partial.length,
    importedNames,
  };
}

function createImportCompletionSource(options = {}) {
  const specifiers = collectImportSpecifiers(options);
  if (!specifiers.length) {
    return null;
  }

  return (context) => {
    const line = context.state.doc.lineAt(context.pos);
    const before = line.text.slice(0, context.pos - line.from);
    const prefix = extractImportSpecifierPrefix(before);
    if (prefix == null) {
      return null;
    }

    const options = buildImportCompletionOptions(prefix, specifiers);
    if (!options.length) {
      return null;
    }

    return {
      from: context.pos - prefix.length,
      options,
      validFor: /[@/\w.-]*/,
    };
  };
}

function completionTypeForSymbolKind(kind) {
  switch (kind) {
    case "Function":
    case "function":
    case "fn":
      return "function";
    case "Class":
    case "class":
      return "class";
    case "Type":
    case "Interface":
    case "type":
    case "interface":
      return "type";
    default:
      return "variable";
  }
}

function createImportSymbolCompletionSource(options = {}) {
  const projectImportIndex = createProjectImportIndex(options.projectFiles);
  const outlineLoader = createOutlineLoader(options);
  if (!outlineLoader) {
    return null;
  }

  return async (context) => {
    const importContext = findNamedImportContext(context.state, context.pos);
    if (!importContext) {
      return null;
    }

    const target = resolveImportTarget(importContext.specifier, options, projectImportIndex);
    if (!target || target.kind !== "project") {
      return null;
    }

    const outline = await outlineLoader(target.relPath);
    const symbols = Array.isArray(outline?.symbols) ? outline.symbols : [];
    const optionsList = symbols
      .filter((symbol) => symbol?.is_exported && !symbol?.is_default && symbol?.kind !== "Import")
      .filter((symbol) => !importContext.importedNames.includes(symbol.name) || symbol.name === importContext.partial)
      .filter((symbol) => !importContext.partial || String(symbol.name || "").startsWith(importContext.partial))
      .map((symbol) => ({
        label: symbol.name,
        type: completionTypeForSymbolKind(symbol.kind),
        detail: `${symbol.kind.toLowerCase()} · ${target.relPath}`,
        info: `line ${symbol.line} · ${target.relPath}`,
      }))
      .sort((a, b) => a.label.localeCompare(b.label));

    if (!optionsList.length) {
      return null;
    }

    return {
      from: importContext.from,
      options: optionsList,
      validFor: /[A-Za-z_$][\w$]*/,
    };
  };
}

function createToolCompletionSource() {
  return (context) => {
    const line = context.state.doc.lineAt(context.pos);
    const before = line.text.slice(0, context.pos - line.from);

    if (/(?:^|[^\w$])Tool\.$/.test(before)) {
      return {
        from: context.pos,
        options: TOOL_NAMESPACE_OPTIONS,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const namespaceMatch = before.match(/(?:^|[^\w$])Tool\.([A-Za-z_$][\w$]*)$/);
    if (namespaceMatch) {
      const partial = namespaceMatch[1];
      const options = TOOL_NAMESPACE_OPTIONS.filter((option) => option.label.startsWith(partial));
      if (!options.length) {
        return null;
      }
      return {
        from: context.pos - partial.length,
        options,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const memberDotMatch = before.match(/(?:^|[^\w$])Tool\.([A-Za-z_$][\w$]*)\.$/);
    if (memberDotMatch) {
      const namespace = memberDotMatch[1];
      const options = TOOL_MEMBER_OPTIONS[namespace];
      if (!options || !options.length) {
        return null;
      }
      return {
        from: context.pos,
        options,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const memberMatch = before.match(/(?:^|[^\w$])Tool\.([A-Za-z_$][\w$]*)\.([A-Za-z_$][\w$]*)$/);
    if (memberMatch) {
      const namespace = memberMatch[1];
      const partial = memberMatch[2];
      const options = (TOOL_MEMBER_OPTIONS[namespace] || []).filter((option) =>
        option.label.startsWith(partial)
      );
      if (!options.length) {
        return null;
      }
      return {
        from: context.pos - partial.length,
        options,
        validFor: /[A-Za-z_$][\w$]*/,
      };
    }

    const word = context.matchBefore(/[A-Za-z_$][\w$]*/);
    if (!word || (word.from === word.to && !context.explicit)) {
      return null;
    }

    if (!"Tool".startsWith(word.text) && word.text !== "Tool") {
      return null;
    }

    return {
      from: word.from,
      options: [
        {
          label: "Tool",
          type: "namespace",
          detail: "Built-in Zebflow helper namespaces",
          info: "Available namespaces: Tool.time, Tool.arr, Tool.stat, Tool.geo",
        },
      ],
      validFor: /[A-Za-z_$][\w$]*/,
    };
  };
}

function createZebflowCompletionSource(options = {}) {
  const importSource = createImportCompletionSource(options);
  const importSymbolSource = createImportSymbolCompletionSource(options);
  const toolSource = createToolCompletionSource();

  return async (context) => {
    const importResult = importSource ? importSource(context) : null;
    if (importResult) {
      return importResult;
    }
    const importSymbolResult = importSymbolSource ? await importSymbolSource(context) : null;
    if (importSymbolResult) {
      return importSymbolResult;
    }
    return toolSource(context);
  };
}

function createImportNavigationExtensions(options = {}) {
  if (typeof options.onOpenImport !== "function") {
    return [];
  }

  const projectImportIndex = createProjectImportIndex(options.projectFiles);
  const outlineLoader = createOutlineLoader(options);

  function canOpenTarget(target) {
    if (!target) {
      return false;
    }
    if (target.kind === "project") {
      return true;
    }
    return target.kind === "library" && typeof options.onOpenLibraryImport === "function";
  }

  async function resolveOpenTarget(reference) {
    const target = resolveImportTarget(reference.specifier, options, projectImportIndex);
    if (!target) {
      return null;
    }
    if (reference.kind !== "symbol" || target.kind !== "project" || !outlineLoader) {
      return target;
    }

    const outline = await outlineLoader(target.relPath);
    const symbols = Array.isArray(outline?.symbols) ? outline.symbols : [];
    const matched = reference.isDefault
      ? symbols.find((symbol) => symbol?.is_exported && symbol?.is_default)
      : symbols.find((symbol) => symbol?.is_exported && symbol?.name === reference.symbolName);

    if (!matched) {
      return target;
    }

    return {
      ...target,
      line: matched.line,
      symbol: matched.name,
    };
  }

  function openImport(view, pos) {
    const reference = findImportReferenceAt(view.state, pos);
    if (!reference) {
      return false;
    }
    const target = resolveImportTarget(reference.specifier, options, projectImportIndex);
    if (!canOpenTarget(target)) {
      return false;
    }
    void resolveOpenTarget(reference).then((resolvedTarget) => {
      if (!resolvedTarget) {
        return;
      }
      if (resolvedTarget.kind === "library" && typeof options.onOpenLibraryImport === "function") {
        options.onOpenLibraryImport(resolvedTarget);
        return;
      }
      options.onOpenImport(resolvedTarget);
    });
    return true;
  }

  function isPointerImport(view, event) {
    const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
    if (typeof pos !== "number") {
      return false;
    }
    const reference = findImportReferenceAt(view.state, pos);
    if (!reference) {
      return false;
    }
    return canOpenTarget(resolveImportTarget(reference.specifier, options, projectImportIndex));
  }

  return [
    EditorView.domEventHandlers({
      mousedown(event, view) {
        if (!(event.metaKey || event.ctrlKey)) {
          return false;
        }
        const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
        if (typeof pos !== "number") {
          return false;
        }
        if (!openImport(view, pos)) {
          return false;
        }
        event.preventDefault();
        return true;
      },
      mousemove(event, view) {
        view.dom.style.cursor = (event.metaKey || event.ctrlKey) && isPointerImport(view, event)
          ? "pointer"
          : "";
        return false;
      },
      mouseleave(_event, view) {
        view.dom.style.cursor = "";
        return false;
      },
      keydown(event, view) {
        if (event.key !== "F12") {
          return false;
        }
        if (!openImport(view, view.state.selection.main.head)) {
          return false;
        }
        event.preventDefault();
        return true;
      },
    }),
  ];
}

function createEditorShortcutExtensions(options = {}) {
  if (typeof options.onSave !== "function") {
    return [];
  }

  return [
    EditorView.domEventHandlers({
      keydown(event) {
        const isSaveKey =
          (event.metaKey || event.ctrlKey) &&
          !event.shiftKey &&
          !event.altKey &&
          event.key.toLowerCase() === "s";
        if (!isSaveKey) {
          return false;
        }
        event.preventDefault();
        options.onSave();
        return true;
      },
    }),
  ];
}

function createZebflowEditorExtensions(options = {}) {
  const extensions = [basicSetup, oneDark];

  const theme = buildEditorTheme(options);
  if (theme) {
    extensions.push(theme);
  }

  if (options.autocomplete) {
    extensions.push(autocompletion());
  }

  if (options.diagnostics) {
    extensions.push(linter(() => []));
    extensions.push(lintGutter());
  }

  const language = resolveLanguageExtension(options.kind);
  if (language) {
    extensions.push(language);
  }

  if (options.autocomplete && isJavaScriptLikeKind(options.kind)) {
    extensions.push(
      javascriptLanguage.data.of({
        autocomplete: createZebflowCompletionSource(options),
      })
    );
  }

  if (typeof options.onDocumentChange === "function") {
    extensions.push(
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) {
          return;
        }
        options.onDocumentChange(update);
      })
    );
  }

  if (options.readonly) {
    extensions.push(EditorView.editable.of(false));
  }

  if (isJavaScriptLikeKind(options.kind)) {
    extensions.push(...createImportNavigationExtensions(options));
  }

  extensions.push(...createEditorShortcutExtensions(options));

  return extensions;
}

const presets = {
  zebflow(options = {}) {
    return createZebflowEditorExtensions(options);
  },
};

const codemirror = {
  CompletionContext,
  EditorView,
  autocompletion,
  basicSetup,
  css,
  cssLanguage,
  javascript,
  javascriptLanguage,
  lintGutter,
  linter,
  oneDark,
  presets,
  setDiagnostics,
  snippetCompletion,
  createZebflowEditorExtensions,
};

export * from "./codemirror.bundle.mjs";
export { codemirror, createZebflowEditorExtensions, presets };
