//#region site/demo-command.js
var e = /* @__PURE__ */ new Set(/* @__PURE__ */ "arch.b2sum.base32.base64.basename.basenc.cat.cksum.comm.cp.cut.date.dir.dircolors.dirname.echo.expand.factor.false.fmt.fold.head.join.link.ln.ls.md5sum.mkdir.mktemp.mv.nl.nproc.numfmt.od.paste.pathchk.pr.printenv.printf.ptx.pwd.readlink.realpath.rm.rmdir.seq.sha1sum.sha224sum.sha256sum.sha384sum.sha512sum.shuf.sort.sum.tail.touch.tr.true.tsort.tty.uname.unexpand.uniq.unlink.vdir.wc".split(".")), t = /* @__PURE__ */ new Map([
	["cmp", "diffutils"],
	["column", "column"],
	["diff", "diffutils"],
	["find", "find"],
	["grep", "grep"],
	["sed", "sed"]
]), n = /* @__PURE__ */ new Set(["cd"]);
function r(e, t) {
	return e === "grep" ? ["--color=never", ...t] : e === "sed" && (t[0] === "-i" || t[0] === "--in-place") && t[1] && !t[1].startsWith("-") ? [
		t[0],
		"-e",
		...t.slice(1)
	] : t;
}
var i = /[\\`;&$(){}!#\u0000-\u001f\u007f]/, a = /^for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\s+([A-Za-z0-9._/-]*\*[A-Za-z0-9._*-]*)\s*;\s*do\s+cat\s+"\$\1"\s*;\s*done\s*$/;
function o(e) {
	let t = e.match(a);
	if (!t) return null;
	let n = t[2], r = n.split("/"), i = r[0] === "." ? r.slice(1) : r, o = [...n].filter((e) => e === "*").length;
	return n.startsWith("/") || o !== 1 || i.some((e) => !e || e === "." || e === "..") || i.slice(0, -1).some((e) => e.includes("*")) ? null : {
		glob: n,
		type: "for-each-cat"
	};
}
function s(e) {
	let t = [], n = "", r = null, i = !1, a = !1;
	for (let o of e.trim()) r ? o === r ? r = null : n += o : o === "'" || o === "\"" ? (r = o, i = !0) : /\s/.test(o) ? i && (t.push({
		hasUnquotedGlob: a,
		value: n
	}), n = "", i = !1, a = !1) : ((o === "*" || o === "?") && (a = !0), n += o, i = !0);
	if (r) throw Error("The translated command contains an unmatched quote.");
	return i && t.push({
		hasUnquotedGlob: a,
		value: n
	}), t;
}
function c(e, t) {
	return e === "find" ? t.some((e) => [
		"-delete",
		"-exec",
		"-execdir",
		"-fls",
		"-fprint",
		"-fprint0",
		"-fprintf",
		"-ok",
		"-okdir"
	].includes(e)) : e === "sort" && t.some((e) => e === "-o" || e.startsWith("-o") || e === "--output" || e.startsWith("--output=") || e === "--compress-program" || e.startsWith("--compress-program="));
}
function l(r) {
	if (typeof r != "string" || !r.trim() || r.includes("||") || i.test(r)) throw Error("That command is outside the browser toolbox.");
	let a = r.split("|");
	return a.map((r, i) => {
		let o = s(r);
		if (o.some(({ value: e }) => e.includes("<") && e !== "<" || e.includes(">") && e !== ">")) throw Error("Only simple file redirection is available.");
		let l = [], u = null, d = null;
		for (let e = 0; e < o.length; e += 1) {
			let t = o[e].value;
			if (t !== "<" && t !== ">") {
				l.push(o[e]);
				continue;
			}
			let n = o[e + 1]?.value, r = t === ">";
			if (l.length === 0 || !n || n === "<" || n === ">" || n.startsWith("/") || n.split("/").some((e) => !e || e === "..") || (r ? d !== null : u !== null) || r && i !== a.length - 1) throw Error("Only simple file redirection is available.");
			r ? d = n : u = n, e += 1;
		}
		let [f, ...p] = l, m = f?.value, h = p.map(({ value: e }) => e), g = p.flatMap((e, t) => e.hasUnquotedGlob ? [t] : []), _ = e.has(m) || t.has(m) || n.has(m);
		if (!m || !_ || c(m, h) || m === "cd" && (a.length !== 1 || u !== null || d !== null || h.length > 1 || h[0]?.startsWith("-"))) throw Error("That command is outside the browser toolbox.");
		return {
			args: h,
			...g.length ? { globIndexes: g } : {},
			inputPath: u,
			name: m,
			outputPath: d
		};
	});
}
function u(e) {
	if (typeof e != "string" || !e.trim()) throw Error("That command is outside the browser toolbox.");
	return o(e.trim()) || {
		pipeline: l(e),
		type: "pipeline"
	};
}
function d(e) {
	try {
		return u(e), !0;
	} catch {
		return !1;
	}
}
//#endregion
export { n as DEMO_BUILTIN_COMMANDS, e as DEMO_COREUTILS_COMMANDS, t as DEMO_STANDALONE_COMMANDS, r as demoRuntimeArguments, d as isAllowedDemoCommand, u as parseDemoCommand, l as parseDemoPipeline };

//# sourceMappingURL=demo-command-v6.js.map