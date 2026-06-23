import { Component, type ErrorInfo, type ReactNode } from "react";
import { LOCALE_STORAGE_KEY, normalizeLocale, translate, type TranslationKey } from "./i18n";

type AppErrorBoundaryProps = {
  children: ReactNode;
};

type AppErrorBoundaryState = {
  error: Error | null;
};

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(t("app.bootstrapErrorLog"), error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <main className="app-fatal-screen" role="alert">
        <section className="app-fatal-panel">
          <p className="eyebrow">{t("fatal.eyebrow")}</p>
          <h1>{t("fatal.title")}</h1>
          <pre>{this.state.error.message}</pre>
          <button
            className="secondary-button"
            type="button"
            onClick={() => window.location.reload()}
          >
            {t("fatal.reload")}
          </button>
        </section>
      </main>
    );
  }
}

function t(key: TranslationKey) {
  const locale =
    typeof window === "undefined"
      ? "en"
      : normalizeLocale(window.localStorage.getItem(LOCALE_STORAGE_KEY));
  return translate(locale, key);
}
