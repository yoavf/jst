//#region site/demo-runtime.js
var e = class {
	constructor({ onProgress: e } = {}) {
		this.frame = null, this.bootPromise = null, this.runReject = null, this.onProgress = e, this.channel = null, this.messageHandler = null;
	}
	boot() {
		return this.bootPromise ? this.bootPromise : (this.channel = crypto.randomUUID(), this.frame = document.createElement("iframe"), this.frame.hidden = !0, this.frame.tabIndex = -1, this.frame.setAttribute("aria-hidden", "true"), this.frame.src = `/demo-sandbox.html?v=15#${this.channel}`, document.body.append(this.frame), this.bootPromise = new Promise((e, t) => {
			let n = !1, r = window.setTimeout(() => {
				i(/* @__PURE__ */ Error("The sandbox took too long to start."));
			}, 45e3), i = (e) => {
				n || (n = !0, window.clearTimeout(r), this.destroy(), t(e instanceof Error ? e : /* @__PURE__ */ Error("The sandbox failed to start.")));
			};
			this.messageHandler = (t) => {
				if (!(t.source !== this.frame?.contentWindow || t.origin !== location.origin || t.data?.channel !== this.channel)) if (t.data?.type === "loaded") this.frame.contentWindow.postMessage({
					type: "boot",
					channel: this.channel
				}, location.origin);
				else if (t.data?.type === "progress") this.onProgress?.(t.data.stage);
				else if (t.data?.type === "ready") {
					if (n) return;
					n = !0, window.clearTimeout(r), window.removeEventListener("message", this.messageHandler), this.messageHandler = null, e();
				} else t.data?.type === "boot-error" && i(Error(t.data.message));
			}, window.addEventListener("message", this.messageHandler), this.frame.addEventListener("error", i, { once: !0 });
		}), this.bootPromise);
	}
	async run(e) {
		if (await this.boot(), !this.frame?.contentWindow) throw Error("The sandbox is not available.");
		return new Promise((t, n) => {
			let r = 0, i = window.setTimeout(() => {
				this.frame?.contentWindow?.postMessage({
					type: "destroy",
					channel: this.channel
				}, location.origin), this.destroy(), n(/* @__PURE__ */ Error("The command used too much time, so the sandbox was reset."));
			}, 6e3), a = (e, t, { reset: n = !1 } = {}) => {
				window.clearTimeout(i), this.messageHandler && window.removeEventListener("message", this.messageHandler), this.messageHandler = null, this.runReject = null, n && this.destroy(), e(t);
			};
			this.runReject = (e) => a(n, e, { reset: !0 }), this.messageHandler = (e) => {
				if (!(e.source !== this.frame?.contentWindow || e.origin !== location.origin || e.data?.channel !== this.channel)) if (e.data?.type === "output") {
					if (r += new TextEncoder().encode(e.data.stdout + e.data.stderr).length, r > 32768) {
						a(n, /* @__PURE__ */ Error("The command produced too much output, so the sandbox was reset."), { reset: !0 });
						return;
					}
					a(t, e.data);
				} else e.data?.type === "run-error" && a(n, Error(e.data.message), { reset: !0 });
			}, window.addEventListener("message", this.messageHandler), this.frame.contentWindow.postMessage({
				type: "run",
				command: e,
				channel: this.channel
			}, location.origin);
		});
	}
	isActive() {
		return !!(this.frame?.contentWindow && this.bootPromise);
	}
	destroy() {
		this.messageHandler && window.removeEventListener("message", this.messageHandler), this.frame?.remove(), this.frame = null, this.bootPromise = null, this.runReject = null, this.messageHandler = null, this.channel = null;
	}
};
//#endregion
export { e as DemoRuntime };

//# sourceMappingURL=demo-runtime.js.map