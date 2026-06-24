/* eslint-disable react-refresh/only-export-components -- shared context-menu primitive: hooks + component live together intentionally. */
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

export interface MenuItem {
  label?: string;
  icon?: ReactNode;
  shortcut?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  onSelect?: () => void;
  submenu?: MenuItem[];
}

export interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

export function useContextMenu() {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const open = useCallback(
    (
      event: { preventDefault: () => void; clientX: number; clientY: number },
      items: MenuItem[],
    ) => {
      event.preventDefault();
      setMenu({ x: event.clientX, y: event.clientY, items });
    },
    [],
  );
  const close = useCallback(() => setMenu(null), []);
  return { menu, open, close };
}

function MenuRow({ item, onClose }: { item: MenuItem; onClose: () => void }) {
  const [openSub, setOpenSub] = useState(false);
  if (item.separator) return <div className="context-menu-sep" role="separator" />;
  const hasSub = Boolean(item.submenu && item.submenu.length);
  return (
    <div
      className="context-menu-row"
      onMouseEnter={() => setOpenSub(true)}
      onMouseLeave={() => setOpenSub(false)}
    >
      <button
        type="button"
        className={["context-menu-item", item.danger ? "danger" : ""].filter(Boolean).join(" ")}
        disabled={item.disabled}
        onClick={() => {
          if (hasSub) return;
          item.onSelect?.();
          onClose();
        }}
      >
        {item.icon ? <span className="context-menu-icon">{item.icon}</span> : null}
        <span className="context-menu-label">{item.label}</span>
        {item.shortcut ? <span className="context-menu-shortcut">{item.shortcut}</span> : null}
        {hasSub ? <span className="context-menu-caret">›</span> : null}
      </button>
      {hasSub && openSub ? (
        <div className="context-menu context-submenu">
          {item.submenu!.map((sub, index) => (
            <MenuRow key={index} item={sub} onClose={onClose} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function ContextMenu({ x, y, items, onClose }: MenuState & { onClose: () => void }) {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onClose();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onClose, true);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onClose, true);
    };
  }, [onClose]);

  // Clamp to viewport.
  const left = Math.min(x, window.innerWidth - 240);
  const top = Math.min(y, window.innerHeight - items.length * 30 - 16);

  return createPortal(
    <div
      ref={ref}
      className="context-menu"
      style={{ left: Math.max(8, left), top: Math.max(8, top) }}
      role="menu"
      onContextMenu={(event) => event.preventDefault()}
    >
      {items.map((item, index) => (
        <MenuRow key={index} item={item} onClose={onClose} />
      ))}
    </div>,
    document.body,
  );
}

// ---------------------------------------------------------------------------
// Promise-based confirm dialog.
// ---------------------------------------------------------------------------

interface ConfirmOptions {
  title: string;
  body?: string;
  confirmLabel?: string;
  danger?: boolean;
}

interface NoticeOptions {
  title: string;
  body?: string;
  confirmLabel?: string;
}

export function useConfirm() {
  const [state, setState] = useState<(ConfirmOptions & { resolve: (ok: boolean) => void }) | null>(
    null,
  );
  const confirm = useCallback(
    (options: ConfirmOptions) =>
      new Promise<boolean>((resolve) => setState({ ...options, resolve })),
    [],
  );
  const settle = (ok: boolean) => {
    state?.resolve(ok);
    setState(null);
  };
  const dialog = state ? (
    <div className="modal-backdrop elevated" role="presentation">
      <section className="modal-panel confirm-modal" role="dialog" aria-modal="true">
        <div className="modal-heading">
          <div>
            <div className="section-label">Confirmação</div>
            <h2>{state.title}</h2>
            {state.body ? <p>{state.body}</p> : null}
          </div>
        </div>
        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={() => settle(false)}>
            Cancelar
          </button>
          <button
            className={state.danger ? "primary-button danger" : "primary-button"}
            type="button"
            onClick={() => settle(true)}
          >
            {state.confirmLabel ?? "Confirmar"}
          </button>
        </div>
      </section>
    </div>
  ) : null;
  return { confirm, dialog };
}

export function useNotice() {
  const [state, setState] = useState<(NoticeOptions & { resolve: () => void }) | null>(null);
  const notice = useCallback(
    (options: NoticeOptions) => new Promise<void>((resolve) => setState({ ...options, resolve })),
    [],
  );
  const settle = () => {
    state?.resolve();
    setState(null);
  };
  const dialog = state ? (
    <div className="modal-backdrop elevated" role="presentation">
      <section
        className="modal-panel confirm-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="notice-title"
        onKeyDown={(event) => {
          if (event.key === "Escape") settle();
        }}
      >
        <div className="modal-heading">
          <div>
            <div className="section-label">Aviso</div>
            <h2 id="notice-title">{state.title}</h2>
            {state.body ? <p>{state.body}</p> : null}
          </div>
        </div>
        <div className="modal-actions">
          <button className="primary-button" type="button" autoFocus onClick={settle}>
            {state.confirmLabel ?? "Entendi"}
          </button>
        </div>
      </section>
    </div>
  ) : null;
  return { notice, dialog };
}

interface PromptOptions {
  title: string;
  label?: string;
  initial?: string;
  confirmLabel?: string;
}

export function usePrompt() {
  const [state, setState] = useState<
    (PromptOptions & { value: string; resolve: (v: string | null) => void }) | null
  >(null);
  const prompt = useCallback(
    (options: PromptOptions) =>
      new Promise<string | null>((resolve) =>
        setState({ ...options, value: options.initial ?? "", resolve }),
      ),
    [],
  );
  const settle = (value: string | null) => {
    state?.resolve(value);
    setState(null);
  };
  const dialog = state ? (
    <div className="modal-backdrop elevated" role="presentation">
      <section className="modal-panel confirm-modal" role="dialog" aria-modal="true">
        <div className="modal-heading">
          <div>
            <div className="section-label">Entrada</div>
            <h2>{state.title}</h2>
          </div>
        </div>
        <label className="prompt-field">
          {state.label ? <span>{state.label}</span> : null}
          <input
            autoFocus
            value={state.value}
            onChange={(event) =>
              setState((prev) => (prev ? { ...prev, value: event.target.value } : prev))
            }
            onKeyDown={(event) => {
              if (event.key === "Enter" && state.value.trim()) settle(state.value.trim());
              if (event.key === "Escape") settle(null);
            }}
          />
        </label>
        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={() => settle(null)}>
            Cancelar
          </button>
          <button
            className="primary-button"
            type="button"
            disabled={!state.value.trim()}
            onClick={() => settle(state.value.trim())}
          >
            {state.confirmLabel ?? "OK"}
          </button>
        </div>
      </section>
    </div>
  ) : null;
  return { prompt, dialog };
}
