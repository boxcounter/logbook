// Polyfill scrollIntoView for jsdom (not implemented in jsdom)
if (typeof Element !== "undefined" && !Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}
