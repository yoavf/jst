//#region site/demo-cli.js
var e = "0.3.2", t = "Turn plain English into a shell command and run it\n\nUsage: jst [OPTIONS] [PROMPT]...\n\nArguments:\n  [PROMPT]...  What you want to do, in plain English\n\nOptions:\n      --yolo         Skip all safety confirmations\n  -i, --interactive  Review the command before running\n      --dry          Show the generated command without running it\n      --status       Check server health, models, and aggregate usage\n  -h, --help         Print help\n  -V, --version      Print version\n\nExamples:\n  jst show the 10 largest files here\n  jst --dry find files containing BANANA\n  jst -i create a photos folder\n  jst --status\n\nThis playground runs commands only inside its disposable browser sandbox.";
function n(e) {
	let t = [], n = "", r = null, i = !1;
	for (let a of e.trim()) r ? a === r ? r = null : n += a : a === "'" || a === "\"" ? (r = a, i = !0) : /\s/.test(a) ? i &&= (t.push(n), n = "", !1) : (n += a, i = !0);
	return r ? null : (i && t.push(n), t);
}
function r(e) {
	let t = n(e);
	if (!t) return { error: "unexpected argument: unmatched quote" };
	if (t.length === 0) return { action: "help" };
	let r = {
		dry: !1,
		interactive: !1,
		status: !1,
		yolo: !1
	}, i = 0;
	for (; i < t.length;) {
		let e = t[i];
		if (e === "--") {
			i += 1;
			break;
		}
		if (!e.startsWith("-") || e === "-") break;
		if (e === "-h" || e === "--help") return { action: "help" };
		if (e === "-V" || e === "--version") return { action: "version" };
		if (e === "--yolo") r.yolo = !0;
		else if (e === "-i" || e === "--interactive") r.interactive = !0;
		else if (e === "--dry") r.dry = !0;
		else if (e === "--status") r.status = !0;
		else return { error: `unexpected argument '${e}'` };
		i += 1;
	}
	if (r.yolo && (r.interactive || r.dry)) return { error: "'--yolo' cannot be used with '--interactive' or '--dry'" };
	if (r.interactive && r.dry) return { error: "'--interactive' cannot be used with '--dry'" };
	let a = t.slice(i);
	return r.status ? r.yolo || r.interactive || r.dry || a.length > 0 ? { error: "'--status' cannot be combined with a prompt or execution options" } : { action: "status" } : a.length === 0 ? { error: "a prompt is required with execution options" } : {
		action: "translate",
		input: a.join(" "),
		...r
	};
}
//#endregion
export { t as JST_HELP, e as JST_VERSION, r as parseJstInvocation };

//# sourceMappingURL=demo-cli.js.map