import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  ConsoleStdout,
  Directory,
  File,
  OpenFile,
  PreopenDirectory,
  WASI,
  WASIProcExit,
  wasi as wasiDefinitions,
} from "@bjorn3/browser_wasi_shim";
import {
  expandWorkspaceArguments,
  expandWorkspaceGlob,
  readWorkspaceFile,
  writeWorkspaceFile,
} from "./demo-filesystem.js";
import { parseDemoCommand } from "./demo-command.js";

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const environment = ["LANG=C.UTF-8", "LC_ALL=C.UTF-8", "NO_COLOR=1", "TERM=dumb"];

test("expands a relative file glob inside the workspace in lexical order", () => {
  const workspace = new PreopenDirectory(".", [
    [
      "museum",
      new Directory([
        ["right.txt", new File(encoder.encode("right\n"))],
        ["core.txt", new File(encoder.encode("core\n"))],
        ["notes.csv", new File(encoder.encode("notes\n"))],
        ["wing.txt", new Directory([])],
      ]),
    ],
  ]);

  assert.deepEqual(expandWorkspaceGlob(workspace, "museum/*.txt"), [
    "museum/core.txt",
    "museum/right.txt",
    "museum/wing.txt",
  ]);
});

test("expands hidden files and directories for the shell glob .??*", () => {
  const workspace = new PreopenDirectory(".", [
    [".vault", new Directory([])],
    [".clues", new Directory([])],
    [".jst_history", new File(encoder.encode("history\n"))],
    [".x", new File(encoder.encode("too short\n"))],
    ["README.md", new File(encoder.encode("visible\n"))],
  ]);

  assert.deepEqual(expandWorkspaceGlob(workspace, ".??*"), [
    ".clues",
    ".jst_history",
    ".vault",
  ]);
});

test("runs JST's bounded cat loop against the in-memory filesystem", async () => {
  const coreutils = await moduleFrom("../docs/assets/uutils/uutils.wasm");
  const workspace = new PreopenDirectory(".", [
    [
      "museum",
      new Directory([
        ["right.txt", new File(encoder.encode("right\n"))],
        ["core.txt", new File(encoder.encode("core\n"))],
      ]),
    ],
  ]);
  const parsed = parseDemoCommand(
    'for file in museum/*.txt; do cat "$file"; done',
  );
  const matches = expandWorkspaceGlob(workspace, parsed.glob);
  const output = await runWasi(
    coreutils,
    ["coreutils", "cat", ...matches],
    "",
    workspace,
  );

  assert.equal(output.code, 0);
  assert.equal(output.stderr, "");
  assert.equal(output.stdout, "core\nright\n");
});

test("runs ls against an expanded hidden-file glob", async () => {
  const coreutils = await moduleFrom("../docs/assets/uutils/uutils.wasm");
  const workspace = new PreopenDirectory(".", [
    [".vault", new Directory([])],
    [".clues", new Directory([])],
    [".jst_history", new File(encoder.encode("history\n"))],
    ["README.md", new File(encoder.encode("visible\n"))],
  ]);
  const parsed = parseDemoCommand("ls -d .??*").pipeline[0];
  const args = expandWorkspaceArguments(
    workspace,
    parsed.args,
    parsed.globIndexes,
  );
  const output = await runWasi(
    coreutils,
    ["coreutils", parsed.name, ...args],
    "",
    workspace,
    false,
  );

  assert.equal(output.code, 0);
  assert.equal(output.stderr, "");
  assert.equal(output.stdout, ".clues\n.jst_history\n.vault\n");
});

async function moduleFrom(path) {
  return WebAssembly.compile(await readFile(new URL(path, import.meta.url)));
}

async function runWasi(module, argv, stdin, workspace, stdoutIsTerminal = true) {
  const stdoutChunks = [];
  const stderrChunks = [];
  const stdoutFile = new File(new Uint8Array());
  const fds = [
    new OpenFile(new File(encoder.encode(stdin), { readonly: true })),
    stdoutIsTerminal
      ? new ConsoleStdout((bytes) => stdoutChunks.push(new Uint8Array(bytes)))
      : new OpenFile(stdoutFile),
    new ConsoleStdout((bytes) => stderrChunks.push(new Uint8Array(bytes))),
    workspace,
  ];
  const wasi = new WASI(argv, environment, fds);

  wasi.wasiImport.args_sizes_get = function argsSizesGet(countPointer, sizePointer) {
    const memory = () => new DataView(wasi.inst.exports.memory.buffer);
    memory().setUint32(countPointer, argv.length, true);
    const size = argv.reduce(
      (total, argument) => total + encoder.encode(argument).length + 1,
      0,
    );
    memory().setUint32(sizePointer, size, true);
    return 0;
  };
  wasi.wasiImport.environ_sizes_get = function environmentSizesGet(
    countPointer,
    sizePointer,
  ) {
    const memory = () => new DataView(wasi.inst.exports.memory.buffer);
    memory().setUint32(countPointer, environment.length, true);
    const size = environment.reduce(
      (total, variable) => total + encoder.encode(variable).length + 1,
      0,
    );
    memory().setUint32(sizePointer, size, true);
    return 0;
  };
  wasiDefinitions.Filestat.prototype.write_bytes = function writeBytes(view, pointer) {
    view.setBigUint64(pointer, this.dev, true);
    view.setBigUint64(pointer + 8, this.ino, true);
    view.setBigUint64(pointer + 16, BigInt(this.filetype), true);
    view.setBigUint64(pointer + 24, this.nlink, true);
    view.setBigUint64(pointer + 32, this.size, true);
    view.setBigUint64(pointer + 40, this.atim, true);
    view.setBigUint64(pointer + 48, this.mtim, true);
    view.setBigUint64(pointer + 56, this.ctim, true);
  };

  let code = 0;
  try {
    const instance = await WebAssembly.instantiate(module, {
      wasi_snapshot_preview1: wasi.wasiImport,
    });
    wasi.start(instance);
  } catch (error) {
    if (error instanceof WASIProcExit) code = error.code;
    else throw error;
  }
  return {
    code,
    stderr: decoder.decode(Buffer.concat(stderrChunks)),
    stdout: stdoutIsTerminal
      ? decoder.decode(Buffer.concat(stdoutChunks))
      : decoder.decode(stdoutFile.data),
  };
}

test("passes ls output to grep through sandbox stdin", async () => {
  const [coreutils, grep] = await Promise.all([
    moduleFrom("../docs/assets/uutils/uutils.wasm"),
    moduleFrom("../docs/assets/uutils/grep.wasm"),
  ]);
  const workspace = new PreopenDirectory(".", [
    ["README.md", new File(encoder.encode("hello\n"), { readonly: true })],
    ["garden", new Directory([])],
  ]);

  const listing = await runWasi(
    coreutils,
    ["coreutils", "ls", "-p"],
    "",
    workspace,
    false,
  );
  assert.equal(listing.code, 0);
  assert.equal(listing.stdout, "README.md\ngarden/\n");

  const filtered = await runWasi(
    grep,
    ["grep", "--color=never", "-v", "/"],
    listing.stdout,
    workspace,
  );
  assert.equal(filtered.code, 0);
  assert.equal(filtered.stderr, "");
  assert.equal(filtered.stdout, "README.md\n");
});

test("solves the four-call lost-process quest with the shipped WASM tools", async () => {
  const [coreutils, grep] = await Promise.all([
    moduleFrom("../docs/assets/uutils/uutils.wasm"),
    moduleFrom("../docs/assets/uutils/grep.wasm"),
  ]);
  const workspace = new PreopenDirectory(".", [
    [
      "README.md",
      new File(
        encoder.encode(
          "PID 31337 is missing.\nLast heartbeat: 0xC0FFEE.\nFind where it was logged.\n",
        ),
        { readonly: true },
      ),
    ],
    [
      "logs",
      new Directory([
        [
          "kernel.log",
          new File(
            encoder.encode(
              "heartbeat=0xC0FFEE payload=messages/core.b64\n",
            ),
            { readonly: true },
          ),
        ],
      ]),
    ],
    [
      "messages",
      new Directory([
        [
          "core.b64",
          new File(
            encoder.encode(
              "SlNUX1FVRVNUX0NPTVBMRVRFX1YxCg==\n",
            ),
            { readonly: true },
          ),
        ],
        [
          "URGENT_DO_NOT_DECODE.b64",
          new File(encoder.encode("SlNUX1JJQ0tST0xMX1YxCg==\n"), {
            readonly: true,
          }),
        ],
      ]),
    ],
  ]);

  const listing = await runWasi(
    coreutils,
    ["coreutils", "ls", "-p"],
    "",
    workspace,
    false,
  );
  const filesOnly = await runWasi(
    grep,
    ["grep", "--color=never", "-v", "/"],
    listing.stdout,
    workspace,
  );
  assert.equal(filesOnly.stdout, "README.md\n");

  const readme = await runWasi(
    coreutils,
    ["coreutils", "cat", "README.md"],
    "",
    workspace,
  );
  assert.match(readme.stdout, /0xC0FFEE/);

  const marker = await runWasi(
    grep,
    ["grep", "--color=never", "-R", "0xC0FFEE", "."],
    "",
    workspace,
  );
  assert.match(marker.stdout, /payload=messages\/core\.b64/);

  const decodedResult = await runWasi(
    coreutils,
    ["coreutils", "base64", "-d", "messages/core.b64"],
    "",
    workspace,
  );
  assert.equal(
    decodedResult.stdout,
    "JST_QUEST_COMPLETE_V1\n",
  );

  const rickroll = await runWasi(
    coreutils,
    ["coreutils", "base64", "-d", "messages/URGENT_DO_NOT_DECODE.b64"],
    "",
    workspace,
    false,
  );
  assert.equal(rickroll.stdout, "JST_RICKROLL_V1\n");
  writeWorkspaceFile(
    workspace,
    "messages/URGENT_DO_NOT_DECODE.txt",
    rickroll.stdout,
  );
  assert.equal(
    readWorkspaceFile(workspace, "messages/URGENT_DO_NOT_DECODE.txt"),
    "JST_RICKROLL_V1\n",
  );
});
