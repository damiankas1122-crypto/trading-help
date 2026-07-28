import { Component, type ErrorInfo, type ReactNode } from "react";
import { formatErrorMessage } from "../utils/format";

// error boundary musi być klasą - React nie ma hookowego odpowiednika
// (getDerivedStateFromError/componentDidCatch). Łapie tylko błędy renderu w
// drzewie poniżej, nie błędy w handlerach zdarzeń (te już mają swój
// try/catch per komponent - patrz CODE_REVIEW F-07)
export class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Nieobsłużony błąd renderu:", error, info.componentStack);
  }

  render() {
    if (!this.state.error) {
      return this.props.children;
    }

    return (
      <div className="h-screen w-screen bg-term-bg text-term-text flex items-center justify-center p-6 font-mono">
        <div className="max-w-md w-full bg-term-panel border border-term-red/50 p-8 space-y-4">
          <h1 className="text-term-red text-lg font-black tracking-[0.1em] uppercase">
            Coś poszło nie tak
          </h1>
          <p className="text-term-dim text-sm leading-relaxed">
            Aplikacja napotkała nieoczekiwany błąd i nie może kontynuować w tym widoku.
            Odśwież aplikację - zapisane dane (klucz API, historia taktyk) nie zostały utracone.
          </p>
          <p className="text-term-faint text-[11px] whitespace-pre-wrap border-t border-term-line pt-3">
            {formatErrorMessage(this.state.error.message)}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="w-full px-4 py-2.5 bg-term-amber/10 border border-term-amber text-term-amber text-xs font-bold uppercase tracking-wider hover:bg-term-amber/20 transition-colors"
          >
            Odśwież aplikację
          </button>
        </div>
      </div>
    );
  }
}
