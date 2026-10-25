// A click on a control that something late just moved does nothing (spec N82, DECISIONS
// B256). A banner that lands a second after the popover opens, Update coming into its row:
// whatever was under the pointer is now something else, and a click meant for it would go
// to what took its place. So for half a second after the page changes by itself, a pointer
// click on a control that moved, or that came in where it is, is not taken; the next one is.
// What the person's own click or key just changed (a menu they opened, a tab, the Scene
// answering them) is theirs, and the keyboard is never held back: focus doesn't move with
// the layout. Nothing on screen says so — the look is unchanged.

/** How long a moved control waits before it takes a click (N82). */
export const STEADY_MS = 500;
/** A change this soon after the person's own click or key is theirs. */
const OWN_MS = 300;
/** What takes a click. */
const CONTROLS = "button, input, select, textarea, a[href], label, [role='menuitemradio'], [role='tab']";

/** A control as it stands: what it is, what it says and where it is. A control that comes
 * back the same in the same place (a re-render) hasn't moved; one that says something else
 * there has. */
function signature(el: Element): string | null {
  const r = el.getBoundingClientRect();
  if (r.width === 0 && r.height === 0) return null;
  const text = (el.getAttribute("aria-label") ?? el.textContent ?? "").trim().slice(0, 80);
  return `${el.tagName}|${text}|${Math.round(r.left)},${Math.round(r.top)},${Math.round(r.width)},${Math.round(r.height)}`;
}

/** Watches the page from now on; returns what stops it. */
export function steady(root: HTMLElement = document.body): () => void {
  let seen = new Set<string>();
  const moved = new WeakMap<Element, number>();
  let own = -Infinity;

  /** Takes the page as it stands; `mark`: what moved or came in did so by itself. */
  function look(mark: boolean) {
    const now = performance.now();
    const next = new Set<string>();
    // The first paint, or a page that had nothing to click, moves nothing.
    const baseline = !mark || seen.size === 0 || now - own < OWN_MS;
    for (const el of root.querySelectorAll(CONTROLS)) {
      const sig = signature(el);
      if (!sig) continue;
      next.add(sig);
      if (!baseline && !seen.has(sig)) moved.set(el, now);
    }
    seen = next;
  }

  const onClick = (e: MouseEvent) => {
    const el = (e.target as Element | null)?.closest?.(CONTROLS);
    // A keyboard's click (Space, Return on a button) carries no pointer count.
    if (e.detail > 0) {
      const at = el ? moved.get(el) : undefined;
      if (at !== undefined && performance.now() - at < STEADY_MS) {
        e.preventDefault();
        e.stopImmediatePropagation();
        return;
      }
    }
    // Only a click that took a control changes anything; one on a line or between controls
    // excuses nothing that lands after it.
    if (el) own = performance.now();
  };
  const onKey = () => {
    own = performance.now();
  };
  // Scrolling and resizing move everything, and nothing lands: the page as it now stands.
  const rebase = () => look(false);

  const watch = new MutationObserver(() => look(true));
  watch.observe(root, { subtree: true, childList: true, characterData: true, attributes: true });
  window.addEventListener("click", onClick, true);
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("scroll", rebase, true);
  window.addEventListener("resize", rebase);
  look(false);
  return () => {
    watch.disconnect();
    window.removeEventListener("click", onClick, true);
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("scroll", rebase, true);
    window.removeEventListener("resize", rebase);
  };
}
