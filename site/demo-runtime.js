const RUN_TIMEOUT_MS = 6_000;
const BOOT_TIMEOUT_MS = 45_000;
const MAX_OUTPUT_BYTES = 32 * 1024;

export class DemoRuntime {
  constructor({ onProgress } = {}) {
    this.frame = null;
    this.bootPromise = null;
    this.runReject = null;
    this.onProgress = onProgress;
    this.channel = null;
    this.messageHandler = null;
  }

  boot() {
    if (this.bootPromise) return this.bootPromise;

    this.channel = crypto.randomUUID();
    this.frame = document.createElement("iframe");
    this.frame.hidden = true;
    this.frame.tabIndex = -1;
    this.frame.setAttribute("aria-hidden", "true");
    this.frame.src = `/demo-sandbox.html?v=14#${this.channel}`;
    document.body.append(this.frame);

    this.bootPromise = new Promise((resolve, reject) => {
      let settled = false;
      const timeout = window.setTimeout(() => {
        fail(new Error("The sandbox took too long to start."));
      }, BOOT_TIMEOUT_MS);
      const fail = (error) => {
        if (settled) return;
        settled = true;
        window.clearTimeout(timeout);
        this.destroy();
        reject(error instanceof Error ? error : new Error("The sandbox failed to start."));
      };
      this.messageHandler = (event) => {
        if (
          event.source !== this.frame?.contentWindow ||
          event.origin !== location.origin ||
          event.data?.channel !== this.channel
        ) {
          return;
        }

        if (event.data?.type === "loaded") {
          this.frame.contentWindow.postMessage(
            { type: "boot", channel: this.channel },
            location.origin,
          );
        } else if (event.data?.type === "progress") {
          this.onProgress?.(event.data.stage);
        } else if (event.data?.type === "ready") {
          if (settled) return;
          settled = true;
          window.clearTimeout(timeout);
          window.removeEventListener("message", this.messageHandler);
          this.messageHandler = null;
          resolve();
        } else if (event.data?.type === "boot-error") {
          fail(new Error(event.data.message));
        }
      };
      window.addEventListener("message", this.messageHandler);
      this.frame.addEventListener("error", fail, { once: true });
    });
    return this.bootPromise;
  }

  async run(command) {
    await this.boot();
    if (!this.frame?.contentWindow) throw new Error("The sandbox is not available.");

    return new Promise((resolve, reject) => {
      let outputBytes = 0;
      const timeout = window.setTimeout(() => {
        this.frame?.contentWindow?.postMessage(
          { type: "destroy", channel: this.channel },
          location.origin,
        );
        this.destroy();
        reject(new Error("The command used too much time, so the sandbox was reset."));
      }, RUN_TIMEOUT_MS);

      const finish = (callback, value, { reset = false } = {}) => {
        window.clearTimeout(timeout);
        if (this.messageHandler) {
          window.removeEventListener("message", this.messageHandler);
        }
        this.messageHandler = null;
        this.runReject = null;
        if (reset) this.destroy();
        callback(value);
      };

      this.runReject = (error) => finish(reject, error, { reset: true });
      this.messageHandler = (event) => {
        if (
          event.source !== this.frame?.contentWindow ||
          event.origin !== location.origin ||
          event.data?.channel !== this.channel
        ) {
          return;
        }

        if (event.data?.type === "output") {
          outputBytes += new TextEncoder().encode(event.data.stdout + event.data.stderr).length;
          if (outputBytes > MAX_OUTPUT_BYTES) {
            finish(
              reject,
              new Error("The command produced too much output, so the sandbox was reset."),
              { reset: true },
            );
            return;
          }
          finish(resolve, event.data);
        } else if (event.data?.type === "run-error") {
          finish(reject, new Error(event.data.message), { reset: true });
        }
      };
      window.addEventListener("message", this.messageHandler);
      this.frame.contentWindow.postMessage(
        { type: "run", command, channel: this.channel },
        location.origin,
      );
    });
  }

  isActive() {
    return Boolean(this.frame?.contentWindow && this.bootPromise);
  }

  destroy() {
    if (this.messageHandler) {
      window.removeEventListener("message", this.messageHandler);
    }
    this.frame?.remove();
    this.frame = null;
    this.bootPromise = null;
    this.runReject = null;
    this.messageHandler = null;
    this.channel = null;
  }
}
