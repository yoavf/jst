//#region site/demo-command.js
var e = /* @__PURE__ */ new Set(/* @__PURE__ */ "arch.b2sum.base32.base64.basename.basenc.cat.cksum.comm.cp.cut.date.dir.dircolors.dirname.echo.expand.factor.false.fmt.fold.head.join.link.ln.ls.md5sum.mkdir.mktemp.mv.nl.nproc.numfmt.od.paste.pathchk.pr.printenv.printf.ptx.pwd.readlink.realpath.rm.rmdir.seq.sha1sum.sha224sum.sha256sum.sha384sum.sha512sum.shuf.sort.sum.tail.touch.tr.true.tsort.tty.uname.unexpand.uniq.unlink.vdir.wc".split(".")), t = /* @__PURE__ */ new Map([
	["cmp", "diffutils"],
	["column", "column"],
	["diff", "diffutils"],
	["find", "find"],
	["grep", "grep"]
]), n = /[\\`;&$(){}!#\u0000-\u001f\u007f]/, r = /^for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\s+([A-Za-z0-9._/-]*\*[A-Za-z0-9._*-]*)\s*;\s*do\s+cat\s+"\$\1"\s*;\s*done\s*$/;
function i(e) {
	let t = e.match(r);
	if (!t) return null;
	let n = t[2], i = n.split("/"), a = i[0] === "." ? i.slice(1) : i, o = [...n].filter((e) => e === "*").length;
	return n.startsWith("/") || o !== 1 || a.some((e) => !e || e === "." || e === "..") || a.slice(0, -1).some((e) => e.includes("*")) ? null : {
		glob: n,
		type: "for-each-cat"
	};
}
function a(e) {
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
function o(e, t) {
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
function s(r) {
	if (typeof r != "string" || !r.trim() || r.includes("||") || n.test(r)) throw Error("That command is outside the browser toolbox.");
	let i = r.split("|");
	return i.map((n, r) => {
		let s = a(n);
		if (s.some(({ value: e }) => e.includes("<") && e !== "<" || e.includes(">") && e !== ">")) throw Error("Only simple file redirection is available.");
		let c = [], l = null, u = null;
		for (let e = 0; e < s.length; e += 1) {
			let t = s[e].value;
			if (t !== "<" && t !== ">") {
				c.push(s[e]);
				continue;
			}
			let n = s[e + 1]?.value, a = t === ">";
			if (c.length === 0 || !n || n === "<" || n === ">" || n.startsWith("/") || n.split("/").some((e) => !e || e === "..") || (a ? u !== null : l !== null) || a && r !== i.length - 1) throw Error("Only simple file redirection is available.");
			a ? u = n : l = n, e += 1;
		}
		let [d, ...f] = c, p = d?.value, m = f.map(({ value: e }) => e), h = f.flatMap((e, t) => e.hasUnquotedGlob ? [t] : []), g = e.has(p) || t.has(p);
		if (!p || !g || o(p, m)) throw Error("That command is outside the browser toolbox.");
		return {
			args: m,
			...h.length ? { globIndexes: h } : {},
			inputPath: l,
			name: p,
			outputPath: u
		};
	});
}
function c(e) {
	if (typeof e != "string" || !e.trim()) throw Error("That command is outside the browser toolbox.");
	return i(e.trim()) || {
		pipeline: s(e),
		type: "pipeline"
	};
}
function l(e) {
	try {
		return c(e), !0;
	} catch {
		return !1;
	}
}
//#endregion
export { e as DEMO_COREUTILS_COMMANDS, t as DEMO_STANDALONE_COMMANDS, l as isAllowedDemoCommand, c as parseDemoCommand, s as parseDemoPipeline };

//# sourceMappingURL=demo-command-v5.js.map