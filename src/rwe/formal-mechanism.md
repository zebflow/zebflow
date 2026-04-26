1. Every internal page transition in RWE after first SSR load and hydration must execute only the irreducible valid state change required to reach the target route, while reusing every unchanged artifact.
2. In RWE, compile must derive only source-dependent artifacts, and render must derive only payload-dependent results from those artifacts.
