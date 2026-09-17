import { getCurrentWindow } from '@tauri-apps/api/window';

let printing = false;
let cleanupTimer: number | null = null;

export function isClinicDeskPrinting() {
  return printing;
}

export async function safePrint() {
  if (printing) return;
  printing = true;
  document.documentElement.dataset.clinicdeskPrinting = 'true';

  const stopEscape = (event: KeyboardEvent) => {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopImmediatePropagation();
  };
  window.addEventListener('keydown', stopEscape, true);

  let unlistenClose: (() => void) | undefined;
  try {
    unlistenClose = await getCurrentWindow().onCloseRequested((event) => {
      if (printing) event.preventDefault();
    });
  } catch {
    // The browser preview can run outside Tauri during development.
  }

  const cleanup = () => {
    if (!printing) return;
    printing = false;
    delete document.documentElement.dataset.clinicdeskPrinting;
    window.removeEventListener('keydown', stopEscape, true);
    unlistenClose?.();
    if (cleanupTimer !== null) window.clearTimeout(cleanupTimer);
    cleanupTimer = null;
  };

  window.addEventListener('afterprint', cleanup, { once: true });
  cleanupTimer = window.setTimeout(cleanup, 120_000);
  requestAnimationFrame(() => window.print());
}
