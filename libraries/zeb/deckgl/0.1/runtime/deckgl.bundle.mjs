function pickDeck(explicit) {
  if (explicit) {
    return explicit;
  }
  if (typeof globalThis !== "undefined") {
    if (globalThis.deck) {
      return globalThis.deck;
    }
    if (globalThis.deckgl) {
      return globalThis.deckgl;
    }
  }
  throw new Error(
    "zeb/deckgl: deck.gl runtime is missing. Provide global deck/deckgl or pass explicit deck namespace.",
  );
}

export function ensureDeck(explicit) {
  return pickDeck(explicit);
}

export function createDeckMapRuntime(host, options = {}) {
  if (!(host instanceof Element)) {
    throw new Error("zeb/deckgl: host element is required");
  }

  const DeckNS = ensureDeck(options.deck);
  const DeckClass =
    DeckNS.Deck || DeckNS.DeckGL || DeckNS.default?.Deck || DeckNS.default?.DeckGL;
  if (!DeckClass) {
    throw new Error("zeb/deckgl: Deck constructor is missing in provided deck namespace");
  }

  const ScatterplotLayer = DeckNS.ScatterplotLayer || DeckNS.default?.ScatterplotLayer;
  const points = Array.isArray(options.points)
    ? options.points
    : [
        { position: [-122.45, 37.78], color: [0, 180, 255], radius: 220 },
        { position: [-122.43, 37.76], color: [255, 100, 120], radius: 200 },
      ];

  const layers = [];
  if (ScatterplotLayer) {
    layers.push(
      new ScatterplotLayer({
        id: options.layerId || "zeb-deck-points",
        data: points,
        getPosition: (d) => d.position,
        getFillColor: (d) => d.color || [60, 180, 255],
        getRadius: (d) => d.radius || 160,
        pickable: true,
      }),
    );
  }

  const deck = new DeckClass({
    parent: host,
    controller: options.controller ?? true,
    layers,
    initialViewState: options.initialViewState || {
      longitude: -122.44,
      latitude: 37.77,
      zoom: 11,
      pitch: 35,
      bearing: 0,
    },
    ...options.deckProps,
  });

  return {
    deck,
    update(next = {}) {
      if (next.initialViewState || next.layers) {
        deck.setProps(next);
      }
    },
    destroy() {
      deck.finalize();
    },
  };
}

export function mountDeckMap(host, options = {}) {
  const runtime = createDeckMapRuntime(host, options);
  return {
    host,
    ...runtime,
  };
}

export const deckgl = {
  ensureDeck,
  createDeckMapRuntime,
  mountDeckMap,
};
