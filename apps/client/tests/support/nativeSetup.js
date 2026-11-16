// The native Jest project (component tests under Testing Library) runs in Node without a DOM; the
// locale store writes through `window.localStorage`, so give it an in-memory one.
const data = new Map();
global.localStorage = {
  getItem: (k) => (data.has(k) ? data.get(k) : null),
  setItem: (k, v) => void data.set(k, String(v)),
  removeItem: (k) => void data.delete(k),
  clear: () => data.clear(),
};
