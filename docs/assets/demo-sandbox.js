import { DEMO_STANDALONE_COMMANDS as e, parseDemoCommand as t } from "./demo-command.js";
var n = class e {
	static read_bytes(t, n) {
		let r = new e();
		return r.buf = t.getUint32(n, !0), r.buf_len = t.getUint32(n + 4, !0), r;
	}
	static read_bytes_array(t, n, r) {
		let i = [];
		for (let a = 0; a < r; a++) i.push(e.read_bytes(t, n + 8 * a));
		return i;
	}
}, r = class e {
	static read_bytes(t, n) {
		let r = new e();
		return r.buf = t.getUint32(n, !0), r.buf_len = t.getUint32(n + 4, !0), r;
	}
	static read_bytes_array(t, n, r) {
		let i = [];
		for (let a = 0; a < r; a++) i.push(e.read_bytes(t, n + 8 * a));
		return i;
	}
}, i = class {
	head_length() {
		return 24;
	}
	name_length() {
		return this.dir_name.byteLength;
	}
	write_head_bytes(e, t) {
		e.setBigUint64(t, this.d_next, !0), e.setBigUint64(t + 8, this.d_ino, !0), e.setUint32(t + 16, this.dir_name.length, !0), e.setUint8(t + 20, this.d_type);
	}
	write_name_bytes(e, t, n) {
		e.set(this.dir_name.slice(0, Math.min(this.dir_name.byteLength, n)), t);
	}
	constructor(e, t, n, r) {
		let i = new TextEncoder().encode(n);
		this.d_next = e, this.d_ino = t, this.d_namlen = i.byteLength, this.d_type = r, this.dir_name = i;
	}
}, a = class {
	write_bytes(e, t) {
		e.setUint8(t, this.fs_filetype), e.setUint16(t + 2, this.fs_flags, !0), e.setBigUint64(t + 8, this.fs_rights_base, !0), e.setBigUint64(t + 16, this.fs_rights_inherited, !0);
	}
	constructor(e, t) {
		this.fs_rights_base = 0n, this.fs_rights_inherited = 0n, this.fs_filetype = e, this.fs_flags = t;
	}
}, o = class {
	write_bytes(e, t) {
		e.setBigUint64(t, this.dev, !0), e.setBigUint64(t + 8, this.ino, !0), e.setUint8(t + 16, this.filetype), e.setBigUint64(t + 24, this.nlink, !0), e.setBigUint64(t + 32, this.size, !0), e.setBigUint64(t + 38, this.atim, !0), e.setBigUint64(t + 46, this.mtim, !0), e.setBigUint64(t + 52, this.ctim, !0);
	}
	constructor(e, t, n) {
		this.dev = 0n, this.nlink = 0n, this.atim = 0n, this.mtim = 0n, this.ctim = 0n, this.ino = e, this.filetype = t, this.size = n;
	}
}, s = class {
	write_bytes(e, t) {
		e.setUint32(t, this.pr_name.byteLength, !0);
	}
	constructor(e) {
		this.pr_name = new TextEncoder().encode(e);
	}
}, c = class e {
	static dir(t) {
		let n = new e();
		return n.tag = 0, n.inner = new s(t), n;
	}
	write_bytes(e, t) {
		e.setUint32(t, this.tag, !0), this.inner.write_bytes(e, t + 4);
	}
}, l = class {
	enable(e) {
		this.log = u(e === void 0 || e, this.prefix);
	}
	get enabled() {
		return this.isEnabled;
	}
	constructor(e) {
		this.isEnabled = e, this.prefix = "wasi:", this.enable(e);
	}
};
function u(e, t) {
	return e ? console.log.bind(console, "%c%s", "color: #265BA0", t) : () => {};
}
var d = new l(!1), f = class extends Error {
	constructor(e) {
		super("exit with exit code " + e), this.code = e;
	}
}, p = class {
	start(e) {
		this.inst = e;
		try {
			return e.exports._start(), 0;
		} catch (e) {
			if (e instanceof f) return e.code;
			throw e;
		}
	}
	initialize(e) {
		this.inst = e, e.exports._initialize && e.exports._initialize();
	}
	constructor(e, t, i, a = {}) {
		this.args = [], this.env = [], this.fds = [], d.enable(a.debug), this.args = e, this.env = t, this.fds = i;
		let o = this;
		this.wasiImport = {
			args_sizes_get(e, t) {
				let n = new DataView(o.inst.exports.memory.buffer);
				n.setUint32(e, o.args.length, !0);
				let r = 0;
				for (let e of o.args) r += e.length + 1;
				return n.setUint32(t, r, !0), d.log(n.getUint32(e, !0), n.getUint32(t, !0)), 0;
			},
			args_get(e, t) {
				let n = new DataView(o.inst.exports.memory.buffer), r = new Uint8Array(o.inst.exports.memory.buffer), i = t;
				for (let i = 0; i < o.args.length; i++) {
					n.setUint32(e, t, !0), e += 4;
					let a = new TextEncoder().encode(o.args[i]);
					r.set(a, t), n.setUint8(t + a.length, 0), t += a.length + 1;
				}
				return d.enabled && d.log(new TextDecoder("utf-8").decode(r.slice(i, t))), 0;
			},
			environ_sizes_get(e, t) {
				let n = new DataView(o.inst.exports.memory.buffer);
				n.setUint32(e, o.env.length, !0);
				let r = 0;
				for (let e of o.env) r += e.length + 1;
				return n.setUint32(t, r, !0), d.log(n.getUint32(e, !0), n.getUint32(t, !0)), 0;
			},
			environ_get(e, t) {
				let n = new DataView(o.inst.exports.memory.buffer), r = new Uint8Array(o.inst.exports.memory.buffer), i = t;
				for (let i = 0; i < o.env.length; i++) {
					n.setUint32(e, t, !0), e += 4;
					let a = new TextEncoder().encode(o.env[i]);
					r.set(a, t), n.setUint8(t + a.length, 0), t += a.length + 1;
				}
				return d.enabled && d.log(new TextDecoder("utf-8").decode(r.slice(i, t))), 0;
			},
			clock_res_get(e, t) {
				let n;
				switch (e) {
					case 1:
						n = 5000n;
						break;
					case 0:
						n = 1000000n;
						break;
					default: return 52;
				}
				return new DataView(o.inst.exports.memory.buffer).setBigUint64(t, n, !0), 0;
			},
			clock_time_get(e, t, n) {
				let r = new DataView(o.inst.exports.memory.buffer);
				if (e === 0) r.setBigUint64(n, BigInt((/* @__PURE__ */ new Date()).getTime()) * 1000000n, !0);
				else if (e == 1) {
					let e;
					try {
						e = BigInt(Math.round(performance.now() * 1e6));
					} catch {
						e = 0n;
					}
					r.setBigUint64(n, e, !0);
				} else r.setBigUint64(n, 0n, !0);
				return 0;
			},
			fd_advise(e, t, n, r) {
				return o.fds[e] == null ? 8 : 0;
			},
			fd_allocate(e, t, n) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_allocate(t, n);
			},
			fd_close(e) {
				if (o.fds[e] != null) {
					let t = o.fds[e].fd_close();
					return o.fds[e] = void 0, t;
				} else return 8;
			},
			fd_datasync(e) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_sync();
			},
			fd_fdstat_get(e, t) {
				if (o.fds[e] != null) {
					let { ret: n, fdstat: r } = o.fds[e].fd_fdstat_get();
					return r?.write_bytes(new DataView(o.inst.exports.memory.buffer), t), n;
				} else return 8;
			},
			fd_fdstat_set_flags(e, t) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_fdstat_set_flags(t);
			},
			fd_fdstat_set_rights(e, t, n) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_fdstat_set_rights(t, n);
			},
			fd_filestat_get(e, t) {
				if (o.fds[e] != null) {
					let { ret: n, filestat: r } = o.fds[e].fd_filestat_get();
					return r?.write_bytes(new DataView(o.inst.exports.memory.buffer), t), n;
				} else return 8;
			},
			fd_filestat_set_size(e, t) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_filestat_set_size(t);
			},
			fd_filestat_set_times(e, t, n, r) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_filestat_set_times(t, n, r);
			},
			fd_pread(e, t, r, i, a) {
				let s = new DataView(o.inst.exports.memory.buffer), c = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let l = n.read_bytes_array(s, t, r), u = 0;
					for (let t of l) {
						let { ret: n, data: r } = o.fds[e].fd_pread(t.buf_len, i);
						if (n != 0) return s.setUint32(a, u, !0), n;
						if (c.set(r, t.buf), u += r.length, i += BigInt(r.length), r.length != t.buf_len) break;
					}
					return s.setUint32(a, u, !0), 0;
				} else return 8;
			},
			fd_prestat_get(e, t) {
				let n = new DataView(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let { ret: r, prestat: i } = o.fds[e].fd_prestat_get();
					return i?.write_bytes(n, t), r;
				} else return 8;
			},
			fd_prestat_dir_name(e, t, n) {
				if (o.fds[e] != null) {
					let { ret: r, prestat: i } = o.fds[e].fd_prestat_get();
					if (i == null) return r;
					let a = i.inner.pr_name;
					return new Uint8Array(o.inst.exports.memory.buffer).set(a.slice(0, n), t), a.byteLength > n ? 37 : 0;
				} else return 8;
			},
			fd_pwrite(e, t, n, i, a) {
				let s = new DataView(o.inst.exports.memory.buffer), c = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let l = r.read_bytes_array(s, t, n), u = 0;
					for (let t of l) {
						let n = c.slice(t.buf, t.buf + t.buf_len), { ret: r, nwritten: l } = o.fds[e].fd_pwrite(n, i);
						if (r != 0) return s.setUint32(a, u, !0), r;
						if (u += l, i += BigInt(l), l != n.byteLength) break;
					}
					return s.setUint32(a, u, !0), 0;
				} else return 8;
			},
			fd_read(e, t, r, i) {
				let a = new DataView(o.inst.exports.memory.buffer), s = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let c = n.read_bytes_array(a, t, r), l = 0;
					for (let t of c) {
						let { ret: n, data: r } = o.fds[e].fd_read(t.buf_len);
						if (n != 0) return a.setUint32(i, l, !0), n;
						if (s.set(r, t.buf), l += r.length, r.length != t.buf_len) break;
					}
					return a.setUint32(i, l, !0), 0;
				} else return 8;
			},
			fd_readdir(e, t, n, r, i) {
				let a = new DataView(o.inst.exports.memory.buffer), s = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let c = 0;
					for (;;) {
						let { ret: l, dirent: u } = o.fds[e].fd_readdir_single(r);
						if (l != 0) return a.setUint32(i, c, !0), l;
						if (u == null) break;
						if (n - c < u.head_length()) {
							c = n;
							break;
						}
						let d = new ArrayBuffer(u.head_length());
						if (u.write_head_bytes(new DataView(d), 0), s.set(new Uint8Array(d).slice(0, Math.min(d.byteLength, n - c)), t), t += u.head_length(), c += u.head_length(), n - c < u.name_length()) {
							c = n;
							break;
						}
						u.write_name_bytes(s, t, n - c), t += u.name_length(), c += u.name_length(), r = u.d_next;
					}
					return a.setUint32(i, c, !0), 0;
				} else return 8;
			},
			fd_renumber(e, t) {
				if (o.fds[e] != null && o.fds[t] != null) {
					let n = o.fds[t].fd_close();
					return n == 0 ? (o.fds[t] = o.fds[e], o.fds[e] = void 0, 0) : n;
				} else return 8;
			},
			fd_seek(e, t, n, r) {
				let i = new DataView(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let { ret: a, offset: s } = o.fds[e].fd_seek(t, n);
					return i.setBigInt64(r, s, !0), a;
				} else return 8;
			},
			fd_sync(e) {
				return o.fds[e] == null ? 8 : o.fds[e].fd_sync();
			},
			fd_tell(e, t) {
				let n = new DataView(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let { ret: r, offset: i } = o.fds[e].fd_tell();
					return n.setBigUint64(t, i, !0), r;
				} else return 8;
			},
			fd_write(e, t, n, i) {
				let a = new DataView(o.inst.exports.memory.buffer), s = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let c = r.read_bytes_array(a, t, n), l = 0;
					for (let t of c) {
						let n = s.slice(t.buf, t.buf + t.buf_len), { ret: r, nwritten: c } = o.fds[e].fd_write(n);
						if (r != 0) return a.setUint32(i, l, !0), r;
						if (l += c, c != n.byteLength) break;
					}
					return a.setUint32(i, l, !0), 0;
				} else return 8;
			},
			path_create_directory(e, t, n) {
				let r = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let i = new TextDecoder("utf-8").decode(r.slice(t, t + n));
					return o.fds[e].path_create_directory(i);
				} else return 8;
			},
			path_filestat_get(e, t, n, r, i) {
				let a = new DataView(o.inst.exports.memory.buffer), s = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let c = new TextDecoder("utf-8").decode(s.slice(n, n + r)), { ret: l, filestat: u } = o.fds[e].path_filestat_get(t, c);
					return u?.write_bytes(a, i), l;
				} else return 8;
			},
			path_filestat_set_times(e, t, n, r, i, a, s) {
				let c = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let l = new TextDecoder("utf-8").decode(c.slice(n, n + r));
					return o.fds[e].path_filestat_set_times(t, l, i, a, s);
				} else return 8;
			},
			path_link(e, t, n, r, i, a, s) {
				let c = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null && o.fds[i] != null) {
					let l = new TextDecoder("utf-8").decode(c.slice(n, n + r)), u = new TextDecoder("utf-8").decode(c.slice(a, a + s)), { ret: d, inode_obj: f } = o.fds[e].path_lookup(l, t);
					return f == null ? d : o.fds[i].path_link(u, f, !1);
				} else return 8;
			},
			path_open(e, t, n, r, i, a, s, c, l) {
				let u = new DataView(o.inst.exports.memory.buffer), f = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let p = new TextDecoder("utf-8").decode(f.slice(n, n + r));
					d.log(p);
					let { ret: m, fd_obj: h } = o.fds[e].path_open(t, p, i, a, s, c);
					if (m != 0) return m;
					o.fds.push(h);
					let g = o.fds.length - 1;
					return u.setUint32(l, g, !0), 0;
				} else return 8;
			},
			path_readlink(e, t, n, r, i, a) {
				let s = new DataView(o.inst.exports.memory.buffer), c = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let l = new TextDecoder("utf-8").decode(c.slice(t, t + n));
					d.log(l);
					let { ret: u, data: f } = o.fds[e].path_readlink(l);
					if (f != null) {
						let e = new TextEncoder().encode(f);
						if (e.length > i) return s.setUint32(a, 0, !0), 8;
						c.set(e, r), s.setUint32(a, e.length, !0);
					}
					return u;
				} else return 8;
			},
			path_remove_directory(e, t, n) {
				let r = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let i = new TextDecoder("utf-8").decode(r.slice(t, t + n));
					return o.fds[e].path_remove_directory(i);
				} else return 8;
			},
			path_rename(e, t, n, r, i, a) {
				let s = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null && o.fds[r] != null) {
					let c = new TextDecoder("utf-8").decode(s.slice(t, t + n)), l = new TextDecoder("utf-8").decode(s.slice(i, i + a)), { ret: u, inode_obj: d } = o.fds[e].path_unlink(c);
					if (d == null) return u;
					if (u = o.fds[r].path_link(l, d, !0), u != 0 && o.fds[e].path_link(c, d, !0) != 0) throw "path_link should always return success when relinking an inode back to the original place";
					return u;
				} else return 8;
			},
			path_symlink(e, t, n, r, i) {
				let a = new Uint8Array(o.inst.exports.memory.buffer);
				return o.fds[n] == null ? 8 : (new TextDecoder("utf-8").decode(a.slice(e, e + t)), new TextDecoder("utf-8").decode(a.slice(r, r + i)), 58);
			},
			path_unlink_file(e, t, n) {
				let r = new Uint8Array(o.inst.exports.memory.buffer);
				if (o.fds[e] != null) {
					let i = new TextDecoder("utf-8").decode(r.slice(t, t + n));
					return o.fds[e].path_unlink_file(i);
				} else return 8;
			},
			poll_oneoff(e, t, n) {
				throw "async io not supported";
			},
			proc_exit(e) {
				throw new f(e);
			},
			proc_raise(e) {
				throw "raised signal " + e;
			},
			sched_yield() {},
			random_get(e, t) {
				let n = new Uint8Array(o.inst.exports.memory.buffer).subarray(e, e + t);
				if ("crypto" in globalThis && !(o.inst.exports.memory.buffer instanceof SharedArrayBuffer)) for (let e = 0; e < t; e += 65536) crypto.getRandomValues(n.subarray(e, e + 65536));
				else for (let e = 0; e < t; e++) n[e] = Math.random() * 256 | 0;
			},
			sock_recv(e, t, n) {
				throw "sockets not supported";
			},
			sock_send(e, t, n) {
				throw "sockets not supported";
			},
			sock_shutdown(e, t) {
				throw "sockets not supported";
			},
			sock_accept(e, t) {
				throw "sockets not supported";
			}
		};
	}
}, m = class {
	fd_allocate(e, t) {
		return 58;
	}
	fd_close() {
		return 0;
	}
	fd_fdstat_get() {
		return {
			ret: 58,
			fdstat: null
		};
	}
	fd_fdstat_set_flags(e) {
		return 58;
	}
	fd_fdstat_set_rights(e, t) {
		return 58;
	}
	fd_filestat_get() {
		return {
			ret: 58,
			filestat: null
		};
	}
	fd_filestat_set_size(e) {
		return 58;
	}
	fd_filestat_set_times(e, t, n) {
		return 58;
	}
	fd_pread(e, t) {
		return {
			ret: 58,
			data: /* @__PURE__ */ new Uint8Array()
		};
	}
	fd_prestat_get() {
		return {
			ret: 58,
			prestat: null
		};
	}
	fd_pwrite(e, t) {
		return {
			ret: 58,
			nwritten: 0
		};
	}
	fd_read(e) {
		return {
			ret: 58,
			data: /* @__PURE__ */ new Uint8Array()
		};
	}
	fd_readdir_single(e) {
		return {
			ret: 58,
			dirent: null
		};
	}
	fd_seek(e, t) {
		return {
			ret: 58,
			offset: 0n
		};
	}
	fd_sync() {
		return 0;
	}
	fd_tell() {
		return {
			ret: 58,
			offset: 0n
		};
	}
	fd_write(e) {
		return {
			ret: 58,
			nwritten: 0
		};
	}
	path_create_directory(e) {
		return 58;
	}
	path_filestat_get(e, t) {
		return {
			ret: 58,
			filestat: null
		};
	}
	path_filestat_set_times(e, t, n, r, i) {
		return 58;
	}
	path_link(e, t, n) {
		return 58;
	}
	path_unlink(e) {
		return {
			ret: 58,
			inode_obj: null
		};
	}
	path_lookup(e, t) {
		return {
			ret: 58,
			inode_obj: null
		};
	}
	path_open(e, t, n, r, i, a) {
		return {
			ret: 54,
			fd_obj: null
		};
	}
	path_readlink(e) {
		return {
			ret: 58,
			data: null
		};
	}
	path_remove_directory(e) {
		return 58;
	}
	path_rename(e, t, n) {
		return 58;
	}
	path_unlink_file(e) {
		return 58;
	}
}, h = class e {
	static issue_ino() {
		return e.next_ino++;
	}
	static root_ino() {
		return 0n;
	}
	constructor() {
		this.ino = e.issue_ino();
	}
};
h.next_ino = 1n;
//#endregion
//#region node_modules/@bjorn3/browser_wasi_shim/dist/fs_mem.js
var g = class extends m {
	fd_allocate(e, t) {
		if (!(this.file.size > e + t)) {
			let n = new Uint8Array(Number(e + t));
			n.set(this.file.data, 0), this.file.data = n;
		}
		return 0;
	}
	fd_fdstat_get() {
		return {
			ret: 0,
			fdstat: new a(4, 0)
		};
	}
	fd_filestat_set_size(e) {
		if (this.file.size > e) this.file.data = new Uint8Array(this.file.data.buffer.slice(0, Number(e)));
		else {
			let t = new Uint8Array(Number(e));
			t.set(this.file.data, 0), this.file.data = t;
		}
		return 0;
	}
	fd_read(e) {
		let t = this.file.data.slice(Number(this.file_pos), Number(this.file_pos + BigInt(e)));
		return this.file_pos += BigInt(t.length), {
			ret: 0,
			data: t
		};
	}
	fd_pread(e, t) {
		return {
			ret: 0,
			data: this.file.data.slice(Number(t), Number(t + BigInt(e)))
		};
	}
	fd_seek(e, t) {
		let n;
		switch (t) {
			case 0:
				n = e;
				break;
			case 1:
				n = this.file_pos + e;
				break;
			case 2:
				n = BigInt(this.file.data.byteLength) + e;
				break;
			default: return {
				ret: 28,
				offset: 0n
			};
		}
		return n < 0 ? {
			ret: 28,
			offset: 0n
		} : (this.file_pos = n, {
			ret: 0,
			offset: this.file_pos
		});
	}
	fd_tell() {
		return {
			ret: 0,
			offset: this.file_pos
		};
	}
	fd_write(e) {
		if (this.file.readonly) return {
			ret: 8,
			nwritten: 0
		};
		if (this.file_pos + BigInt(e.byteLength) > this.file.size) {
			let t = this.file.data;
			this.file.data = new Uint8Array(Number(this.file_pos + BigInt(e.byteLength))), this.file.data.set(t);
		}
		return this.file.data.set(e, Number(this.file_pos)), this.file_pos += BigInt(e.byteLength), {
			ret: 0,
			nwritten: e.byteLength
		};
	}
	fd_pwrite(e, t) {
		if (this.file.readonly) return {
			ret: 8,
			nwritten: 0
		};
		if (t + BigInt(e.byteLength) > this.file.size) {
			let n = this.file.data;
			this.file.data = new Uint8Array(Number(t + BigInt(e.byteLength))), this.file.data.set(n);
		}
		return this.file.data.set(e, Number(t)), {
			ret: 0,
			nwritten: e.byteLength
		};
	}
	fd_filestat_get() {
		return {
			ret: 0,
			filestat: this.file.stat()
		};
	}
	constructor(e) {
		super(), this.file_pos = 0n, this.file = e;
	}
}, _ = class extends m {
	fd_seek(e, t) {
		return {
			ret: 8,
			offset: 0n
		};
	}
	fd_tell() {
		return {
			ret: 8,
			offset: 0n
		};
	}
	fd_allocate(e, t) {
		return 8;
	}
	fd_fdstat_get() {
		return {
			ret: 0,
			fdstat: new a(3, 0)
		};
	}
	fd_readdir_single(e) {
		if (d.enabled && (d.log("readdir_single", e), d.log(e, this.dir.contents.keys())), e == 0n) return {
			ret: 0,
			dirent: new i(1n, this.dir.ino, ".", 3)
		};
		if (e == 1n) return {
			ret: 0,
			dirent: new i(2n, this.dir.parent_ino(), "..", 3)
		};
		if (e >= BigInt(this.dir.contents.size) + 2n) return {
			ret: 0,
			dirent: null
		};
		let [t, n] = Array.from(this.dir.contents.entries())[Number(e - 2n)];
		return {
			ret: 0,
			dirent: new i(e + 1n, n.ino, t, n.stat().filetype)
		};
	}
	path_filestat_get(e, t) {
		let { ret: n, path: r } = b.from(t);
		if (r == null) return {
			ret: n,
			filestat: null
		};
		let { ret: i, entry: a } = this.dir.get_entry_for_path(r);
		return a == null ? {
			ret: i,
			filestat: null
		} : {
			ret: 0,
			filestat: a.stat()
		};
	}
	path_lookup(e, t) {
		let { ret: n, path: r } = b.from(e);
		if (r == null) return {
			ret: n,
			inode_obj: null
		};
		let { ret: i, entry: a } = this.dir.get_entry_for_path(r);
		return a == null ? {
			ret: i,
			inode_obj: null
		} : {
			ret: 0,
			inode_obj: a
		};
	}
	path_open(e, t, n, r, i, a) {
		let { ret: o, path: s } = b.from(t);
		if (s == null) return {
			ret: o,
			fd_obj: null
		};
		let { ret: c, entry: l } = this.dir.get_entry_for_path(s);
		if (l == null) {
			if (c != 44) return {
				ret: c,
				fd_obj: null
			};
			if ((n & 1) == 1) {
				let { ret: e, entry: r } = this.dir.create_entry_for_path(t, (n & 2) == 2);
				if (r == null) return {
					ret: e,
					fd_obj: null
				};
				l = r;
			} else return {
				ret: 44,
				fd_obj: null
			};
		} else if ((n & 4) == 4) return {
			ret: 20,
			fd_obj: null
		};
		return (n & 2) == 2 && l.stat().filetype !== 3 ? {
			ret: 54,
			fd_obj: null
		} : l.path_open(n, r, a);
	}
	path_create_directory(e) {
		return this.path_open(0, e, 3, 0n, 0n, 0).ret;
	}
	path_link(e, t, n) {
		let { ret: r, path: i } = b.from(e);
		if (i == null) return r;
		if (i.is_dir) return 44;
		let { ret: a, parent_entry: o, filename: s, entry: c } = this.dir.get_parent_dir_and_entry_for_path(i, !0);
		if (o == null || s == null) return a;
		if (c != null) {
			let e = t.stat().filetype == 3, r = c.stat().filetype == 3;
			if (e && r) if (n && c instanceof x) {
				if (c.contents.size != 0) return 55;
			} else return 20;
			else if (e && !r) return 54;
			else if (!e && r) return 31;
			else if (!(t.stat().filetype == 4 && c.stat().filetype == 4)) return 20;
		}
		return !n && t.stat().filetype == 3 ? 63 : (o.contents.set(s, t), 0);
	}
	path_unlink(e) {
		let { ret: t, path: n } = b.from(e);
		if (n == null) return {
			ret: t,
			inode_obj: null
		};
		let { ret: r, parent_entry: i, filename: a, entry: o } = this.dir.get_parent_dir_and_entry_for_path(n, !0);
		return i == null || a == null ? {
			ret: r,
			inode_obj: null
		} : o == null ? {
			ret: 44,
			inode_obj: null
		} : (i.contents.delete(a), {
			ret: 0,
			inode_obj: o
		});
	}
	path_unlink_file(e) {
		let { ret: t, path: n } = b.from(e);
		if (n == null) return t;
		let { ret: r, parent_entry: i, filename: a, entry: o } = this.dir.get_parent_dir_and_entry_for_path(n, !1);
		return i == null || a == null || o == null ? r : o.stat().filetype === 3 ? 31 : (i.contents.delete(a), 0);
	}
	path_remove_directory(e) {
		let { ret: t, path: n } = b.from(e);
		if (n == null) return t;
		let { ret: r, parent_entry: i, filename: a, entry: o } = this.dir.get_parent_dir_and_entry_for_path(n, !1);
		return i == null || a == null || o == null ? r : !(o instanceof x) || o.stat().filetype !== 3 ? 54 : o.contents.size === 0 ? i.contents.delete(a) ? 0 : 44 : 55;
	}
	fd_filestat_get() {
		return {
			ret: 0,
			filestat: this.dir.stat()
		};
	}
	fd_filestat_set_size(e) {
		return 8;
	}
	fd_read(e) {
		return {
			ret: 8,
			data: /* @__PURE__ */ new Uint8Array()
		};
	}
	fd_pread(e, t) {
		return {
			ret: 8,
			data: /* @__PURE__ */ new Uint8Array()
		};
	}
	fd_write(e) {
		return {
			ret: 8,
			nwritten: 0
		};
	}
	fd_pwrite(e, t) {
		return {
			ret: 8,
			nwritten: 0
		};
	}
	constructor(e) {
		super(), this.dir = e;
	}
}, v = class extends _ {
	fd_prestat_get() {
		return {
			ret: 0,
			prestat: c.dir(this.prestat_name)
		};
	}
	constructor(e, t) {
		super(new x(t)), this.prestat_name = e;
	}
}, y = class extends h {
	path_open(e, t, n) {
		if (this.readonly && (t & BigInt(64)) == BigInt(64)) return {
			ret: 63,
			fd_obj: null
		};
		if ((e & 8) == 8) {
			if (this.readonly) return {
				ret: 63,
				fd_obj: null
			};
			this.data = new Uint8Array([]);
		}
		let r = new g(this);
		return n & 1 && r.fd_seek(0n, 2), {
			ret: 0,
			fd_obj: r
		};
	}
	get size() {
		return BigInt(this.data.byteLength);
	}
	stat() {
		return new o(this.ino, 4, this.size);
	}
	constructor(e, t) {
		super(), this.data = new Uint8Array(e), this.readonly = !!t?.readonly;
	}
}, b = class e {
	static from(t) {
		let n = new e();
		if (n.is_dir = t.endsWith("/"), t.startsWith("/")) return {
			ret: 76,
			path: null
		};
		if (t.includes("\0")) return {
			ret: 28,
			path: null
		};
		for (let e of t.split("/")) if (!(e === "" || e === ".")) {
			if (e === "..") {
				if (n.parts.pop() == null) return {
					ret: 76,
					path: null
				};
				continue;
			}
			n.parts.push(e);
		}
		return {
			ret: 0,
			path: n
		};
	}
	to_path_string() {
		let e = this.parts.join("/");
		return this.is_dir && (e += "/"), e;
	}
	constructor() {
		this.parts = [], this.is_dir = !1;
	}
}, x = class e extends h {
	parent_ino() {
		return this.parent == null ? h.root_ino() : this.parent.ino;
	}
	path_open(e, t, n) {
		return {
			ret: 0,
			fd_obj: new _(this)
		};
	}
	stat() {
		return new o(this.ino, 3, 0n);
	}
	get_entry_for_path(t) {
		let n = this;
		for (let r of t.parts) {
			if (!(n instanceof e)) return {
				ret: 54,
				entry: null
			};
			let t = n.contents.get(r);
			if (t !== void 0) n = t;
			else return d.log(r), {
				ret: 44,
				entry: null
			};
		}
		return t.is_dir && n.stat().filetype != 3 ? {
			ret: 54,
			entry: null
		} : {
			ret: 0,
			entry: n
		};
	}
	get_parent_dir_and_entry_for_path(t, n) {
		let r = t.parts.pop();
		if (r === void 0) return {
			ret: 28,
			parent_entry: null,
			filename: null,
			entry: null
		};
		let { ret: i, entry: a } = this.get_entry_for_path(t);
		if (a == null) return {
			ret: i,
			parent_entry: null,
			filename: null,
			entry: null
		};
		if (!(a instanceof e)) return {
			ret: 54,
			parent_entry: null,
			filename: null,
			entry: null
		};
		let o = a.contents.get(r);
		return o === void 0 ? n ? {
			ret: 0,
			parent_entry: a,
			filename: r,
			entry: null
		} : {
			ret: 44,
			parent_entry: null,
			filename: null,
			entry: null
		} : t.is_dir && o.stat().filetype != 3 ? {
			ret: 54,
			parent_entry: null,
			filename: null,
			entry: null
		} : {
			ret: 0,
			parent_entry: a,
			filename: r,
			entry: o
		};
	}
	create_entry_for_path(t, n) {
		let { ret: r, path: i } = b.from(t);
		if (i == null) return {
			ret: r,
			entry: null
		};
		let { ret: a, parent_entry: o, filename: s, entry: c } = this.get_parent_dir_and_entry_for_path(i, !0);
		if (o == null || s == null) return {
			ret: a,
			entry: null
		};
		if (c != null) return {
			ret: 20,
			entry: null
		};
		d.log("create", i);
		let l;
		return l = n ? new e(/* @__PURE__ */ new Map()) : new y(/* @__PURE__ */ new ArrayBuffer(0)), o.contents.set(s, l), c = l, {
			ret: 0,
			entry: c
		};
	}
	constructor(t) {
		super(), this.parent = null, t instanceof Array ? this.contents = new Map(t) : this.contents = t;
		for (let t of this.contents.values()) t instanceof e && (t.parent = this);
	}
}, S = class e extends m {
	fd_filestat_get() {
		return {
			ret: 0,
			filestat: new o(this.ino, 2, BigInt(0))
		};
	}
	fd_fdstat_get() {
		let e = new a(2, 0);
		return e.fs_rights_base = BigInt(64), {
			ret: 0,
			fdstat: e
		};
	}
	fd_write(e) {
		return this.write(e), {
			ret: 0,
			nwritten: e.byteLength
		};
	}
	static lineBuffered(t) {
		let n = new TextDecoder("utf-8", { fatal: !1 }), r = "";
		return new e((e) => {
			r += n.decode(e, { stream: !0 });
			let i = r.split("\n");
			for (let [e, n] of i.entries()) e < i.length - 1 ? t(n) : r = n;
		});
	}
	constructor(e) {
		super(), this.ino = h.issue_ino(), this.write = e;
	}
}, C = new TextEncoder(), w = new TextDecoder();
function T(e) {
	let t = e.split("/").filter((e) => e !== ".");
	if (!e || e.startsWith("/") || t.length === 0 || t.some((e) => !e || e === "..")) throw Error(`${e}: not a safe sandbox path`);
	return t;
}
function E(e, t) {
	let n = e.dir;
	for (let e of T(t)) {
		if (!(n instanceof x)) throw Error(`${t}: not a readable sandbox file`);
		if (n = n.contents.get(e), !n) throw Error(`${t}: no such sandbox file`);
	}
	if (!(n instanceof y)) throw Error(`${t}: not a regular file`);
	return w.decode(n.data);
}
function D(e, t, n) {
	let r = T(t), i = r.pop(), a = e.dir;
	for (let e of r) {
		if (!(a instanceof x)) throw Error(`${t}: parent is not a directory`);
		if (a = a.contents.get(e), !a) throw Error(`${t}: parent directory does not exist`);
	}
	if (!(a instanceof x)) throw Error(`${t}: parent is not a directory`);
	if (a.contents.get(i) instanceof x) throw Error(`${t}: is a directory`);
	a.contents.set(i, new y(C.encode(n)));
}
function O(e, t) {
	let n = T(t), r = n.pop();
	if (!/[*?]/.test(r) || n.some((e) => /[*?]/.test(e))) throw Error(`${t}: not a supported sandbox glob`);
	let i = e.dir;
	for (let e of n) if (!(i instanceof x) || (i = i.contents.get(e), !i)) return [];
	if (!(i instanceof x)) return [];
	let a = "";
	for (let e of r) e === "*" ? a += ".*" : e === "?" ? a += "." : a += e.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
	let o = RegExp(`^${a}$`), s = n.length ? `${n.join("/")}/` : "", c = [...i.contents.entries()].filter(([e, t]) => (t instanceof y || t instanceof x) && (r.startsWith(".") || !e.startsWith(".")) && o.test(e)).map(([e]) => `${s}${e}`).sort();
	if (c.length > 128) throw Error(`${t}: matched too many sandbox paths`);
	return c;
}
function k(e, t, n = []) {
	let r = new Set(n);
	return t.flatMap((t, n) => {
		if (!r.has(n)) return [t];
		let i = O(e, t);
		return i.length ? i : [t];
	});
}
//#endregion
//#region site/demo-sandbox.js
var A = 32 * 1024, j = new TextEncoder(), M = new TextDecoder(), N = new URL(location.href).hash.slice(1), P = {
	coreutils: "/assets/uutils/uutils.wasm?v=c868c992",
	column: "/assets/uutils/column.wasm?v=d4ed168b",
	diffutils: "/assets/uutils/diffutils.wasm?v=d4a30573",
	find: "/assets/uutils/find.wasm?v=6664ea48",
	grep: "/assets/uutils/grep.wasm?v=a57e7f1e"
}, F, I;
function L(e) {
	parent.postMessage({
		...e,
		channel: N
	}, location.origin);
}
function R(e) {
	L({
		type: "progress",
		stage: e
	});
}
function z(e) {
	return new y(j.encode(e), { readonly: !0 });
}
function B(e) {
	return new x(new Map(e));
}
function V() {
	let e = new v(".", /* @__PURE__ */ new Map([
		["README.md", z("# JST playground\n\nDisposable in-memory filesystem.\nNo host filesystem. No network.\n\nPID 31337 is missing.\nLast heartbeat: 0xC0FFEE.\nFind where it was logged.\n")],
		["downloads", B([["expenses.csv", z("date,category,amount\n2026-07-03,hosting,24\n2026-07-08,software,12\n2026-07-16,hardware,89\n")]])],
		["logs", B([["kernel.log", z("Jul 27 00:00:01 jst kernel: process=31337 state=missing\nJul 27 00:00:02 jst kernel: heartbeat=0xC0FFEE payload=messages/core.b64\nJul 27 00:00:03 jst kernel: core_dump=0\n")]])],
		["messages", B([["core.b64", z("SlNUX1FVRVNUX0NPTVBMRVRFX1YxCg==\n")], ["URGENT_DO_NOT_DECODE.b64", z("SlNUX1JJQ0tST0xMX1YxCg==\n")]])],
		["museum", B([["left.txt", z("exhibit=404\nbias=left\n")], ["right.txt", z("exhibit=404\nbias=right\n")]])],
		["projects", B([["jst", B([["src", B([["main.rs", z("fn main() {\n    println!(\"jst do this.\");\n}\n")]])]])]])]
	])), t = BigInt(Date.now()) * 1000000n, n = (e) => {
		let r = e.stat.bind(e);
		if (e.stat = () => {
			let e = r();
			return e.atim = t, e.mtim = t, e.ctim = t, e;
		}, e.contents instanceof Map) for (let t of e.contents.values()) n(t);
	};
	return n(e.dir), e;
}
async function H(e) {
	let t = await fetch(e, { cache: "force-cache" });
	if (!t.ok) throw Error(`A Linux tool failed to load (${t.status}).`);
	if (WebAssembly.compileStreaming) try {
		return await WebAssembly.compileStreaming(t.clone());
	} catch {}
	return WebAssembly.compile(await t.arrayBuffer());
}
async function U() {
	R("runtime"), I = V(), R("package");
	let e = await Promise.all(Object.entries(P).map(async ([e, t]) => [e, await H(t)]));
	F = Object.fromEntries(e), R("shell"), R("ready");
}
function W(e) {
	let t = e.reduce((e, t) => e + t.length, 0), n = new Uint8Array(t), r = 0;
	for (let t of e) n.set(t, r), r += t.length;
	return M.decode(n);
}
async function G(e, t, n, r = !0) {
	let i = [], a = [], s = new y(/* @__PURE__ */ new Uint8Array()), c = [
		new g(new y(j.encode(n), { readonly: !0 })),
		r ? new S((e) => i.push(new Uint8Array(e))) : new g(s),
		new S((e) => a.push(new Uint8Array(e))),
		I
	], l = [
		"LANG=C.UTF-8",
		"LC_ALL=C.UTF-8",
		"NO_COLOR=1",
		"TERM=dumb"
	], u = new p(t, l, c);
	u.wasiImport.args_sizes_get = function(e, n) {
		let r = () => new DataView(u.inst.exports.memory.buffer);
		r().setUint32(e, t.length, !0);
		let i = t.reduce((e, t) => e + j.encode(t).length + 1, 0);
		return r().setUint32(n, i, !0), 0;
	}, u.wasiImport.environ_sizes_get = function(e, t) {
		let n = () => new DataView(u.inst.exports.memory.buffer);
		n().setUint32(e, l.length, !0);
		let r = l.reduce((e, t) => e + j.encode(t).length + 1, 0);
		return n().setUint32(t, r, !0), 0;
	}, o.prototype.write_bytes = function(e, t) {
		e.setBigUint64(t, this.dev, !0), e.setBigUint64(t + 8, this.ino, !0), e.setBigUint64(t + 16, BigInt(this.filetype), !0), e.setBigUint64(t + 24, this.nlink, !0), e.setBigUint64(t + 32, this.size, !0), e.setBigUint64(t + 40, this.atim, !0), e.setBigUint64(t + 48, this.mtim, !0), e.setBigUint64(t + 56, this.ctim, !0);
	};
	let d = 0;
	try {
		let t = await WebAssembly.instantiate(e, { wasi_snapshot_preview1: u.wasiImport });
		u.start(t.instance || t);
	} catch (e) {
		if (e instanceof f) d = e.code;
		else throw e;
	}
	return {
		code: d,
		stderr: W(a),
		stdout: r ? W(i) : M.decode(s.data)
	};
}
async function K(n) {
	if (!F || !I) throw Error("The Linux tools did not finish loading.");
	let r = t(n), i = r.type === "for-each-cat" ? O(I, r.glob) : null, a = r.type === "for-each-cat" ? [{
		args: i.length ? i : [r.glob],
		inputPath: null,
		name: "cat",
		outputPath: null
	}] : r.pipeline, o = "", s = "", c = 0, l = null, u = "";
	for (let [t, n] of a.entries()) {
		let { args: r, globIndexes: i, inputPath: d, name: f } = n;
		d && (o = E(I, d));
		let p = e.get(f) || "coreutils", m = k(I, r, i), h = f === "grep" ? ["--color=never", ...m] : m, g = p === "coreutils" ? [
			"coreutils",
			f,
			...h
		] : [f, ...h], _ = await G(F[p], g, o, t === a.length - 1 && !n.outputPath);
		if (c = _.code, o = _.stdout, s += _.stderr, j.encode(o + s).byteLength > A) throw Error("The command produced too much output, so the sandbox was reset.");
		if (n.outputPath && (D(I, n.outputPath, o), l = n.outputPath, u = o, o = ""), c !== 0) break;
	}
	return {
		code: c,
		outputPath: l,
		redirectedStdout: u,
		stderr: s,
		stdout: o
	};
}
window.addEventListener("message", async (e) => {
	if (!(e.source !== parent || e.origin !== location.origin || e.data?.channel !== N)) {
		if (e.data?.type === "boot") try {
			await U(), L({ type: "ready" });
		} catch (e) {
			L({
				type: "boot-error",
				message: e instanceof Error ? e.message : "The sandbox failed to start."
			});
		}
		else if (e.data?.type === "run") try {
			let t = await K(e.data.command);
			L({
				type: "output",
				code: t.code,
				outputPath: t.outputPath,
				redirectedStdout: t.redirectedStdout,
				stderr: t.stderr,
				stdout: t.stdout
			});
		} catch (e) {
			L({
				type: "run-error",
				message: e instanceof Error ? e.message : "The command failed."
			});
		}
	}
}), L({ type: "loaded" });
//#endregion

//# sourceMappingURL=demo-sandbox.js.map