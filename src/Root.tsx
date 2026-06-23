import { useState } from "react";
import { App } from "./App";
import { AppErrorBoundary } from "./AppErrorBoundary";
import { I18nProvider } from "./i18n";
import { SplashScreen } from "./SplashScreen";

export function Root() {
  const [splashDone, setSplashDone] = useState(false);
  return (
    <I18nProvider>
      <AppErrorBoundary>
        <App />
        {splashDone ? null : <SplashScreen onDone={() => setSplashDone(true)} />}
      </AppErrorBoundary>
    </I18nProvider>
  );
}
