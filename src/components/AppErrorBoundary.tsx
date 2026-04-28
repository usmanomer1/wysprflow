import React from "react";

type State = {
  error: Error | null;
  eventMessage: string | null;
};

export class AppErrorBoundary extends React.Component<
  React.PropsWithChildren,
  State
> {
  state: State = {
    error: null,
    eventMessage: null,
  };

  componentDidCatch(error: Error) {
    console.error("AppErrorBoundary caught render error:", error);
    this.setState({ error });
  }

  componentDidMount() {
    window.addEventListener("error", this.handleWindowError);
    window.addEventListener("unhandledrejection", this.handleUnhandledRejection);
  }

  componentWillUnmount() {
    window.removeEventListener("error", this.handleWindowError);
    window.removeEventListener("unhandledrejection", this.handleUnhandledRejection);
  }

  handleWindowError = (event: ErrorEvent) => {
    const message =
      event.error instanceof Error
        ? `${event.error.name}: ${event.error.message}`
        : event.message || "Unknown window error";
    console.error("window error:", event.error ?? event.message);
    this.setState((state) => ({
      error: state.error,
      eventMessage: message,
    }));
  };

  handleUnhandledRejection = (event: PromiseRejectionEvent) => {
    const reason =
      event.reason instanceof Error
        ? `${event.reason.name}: ${event.reason.message}`
        : typeof event.reason === "string"
          ? event.reason
          : JSON.stringify(event.reason, null, 2);
    console.error("unhandled rejection:", event.reason);
    this.setState((state) => ({
      error: state.error,
      eventMessage: reason,
    }));
  };

  render() {
    if (this.state.error || this.state.eventMessage) {
      const message = this.state.error
        ? `${this.state.error.name}: ${this.state.error.message}`
        : this.state.eventMessage ?? "Unknown startup error";

      return (
        <div className="flex min-h-screen items-center justify-center bg-background px-6 py-10 text-foreground">
          <div className="w-full max-w-2xl rounded-lg border border-destructive/30 bg-card p-6 shadow-sm">
            <h1 className="text-lg font-semibold">wysprflow failed to render</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              The frontend hit a startup error instead of mounting the app UI.
            </p>
            <pre className="mt-4 overflow-x-auto rounded-md bg-muted/40 p-3 text-xs whitespace-pre-wrap break-words">
              {message}
            </pre>
            {this.state.error?.stack ? (
              <details className="mt-3">
                <summary className="cursor-pointer text-sm text-muted-foreground">
                  Stack trace
                </summary>
                <pre className="mt-2 overflow-x-auto rounded-md bg-muted/40 p-3 text-xs whitespace-pre-wrap break-words">
                  {this.state.error.stack}
                </pre>
              </details>
            ) : null}
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
