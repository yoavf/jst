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
  DEMO_STANDALONE_COMMANDS,
  demoRuntimeArguments,
  parseDemoCommand,
} from "./demo-command.js";
import {
  expandWorkspaceArguments,
  expandWorkspaceGlob,
  readWorkspaceFile,
  resolveWorkspaceDirectory,
  workspaceAtDirectory,
  writeWorkspaceFile,
} from "./demo-filesystem.js";
import { BoundedOpenFile, OutputBudget } from "./demo-output.js";
import { installWasiRandomGet } from "./demo-wasi.js";

// The execution design is adapted from the MIT-licensed uutils browser
// playground. Pinned source revisions and checksums live beside the binaries
// in docs/assets/uutils/ATTRIBUTION.md.

const MAX_OUTPUT_BYTES = 32 * 1024;
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const isWorker =
  typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope;
const channel = isWorker ? null : new URL(location.href).hash.slice(1);
const moduleUrls = {
  coreutils: "/assets/uutils/uutils.wasm?v=c868c992",
  column: "/assets/uutils/column.wasm?v=d4ed168b",
  diffutils: "/assets/uutils/diffutils.wasm?v=d4a30573",
  find: "/assets/uutils/find.wasm?v=6664ea48",
  grep: "/assets/uutils/grep.wasm?v=a57e7f1e",
  sed: "/assets/uutils/sed.wasm?v=dab89970",
};

let modules;
let workspace;
let workingDirectory;

function post(message) {
  if (isWorker) {
    self.postMessage(message);
  } else {
    parent.postMessage({ ...message, channel }, location.origin);
  }
}

function progress(stage) {
  post({ type: "progress", stage });
}

function file(contents) {
  return new File(encoder.encode(contents), { readonly: true });
}

function directory(entries) {
  return new Directory(new Map(entries));
}

function buildWorkspace() {
  const root = new Map([
    [
      "README.md",
      file(`# JST playground

Disposable in-memory filesystem.
No host filesystem. No network.

PID 31337 is missing.
Last heartbeat: 0xC0FFEE.
Find where it was logged.
`),
    ],
    [
      "downloads",
      directory([
        [
          "expenses.csv",
          file(`date,category,amount
2026-07-03,hosting,24
2026-07-08,software,12
2026-07-16,hardware,89
`),
        ],
      ]),
    ],
    [
      "logs",
      directory([
        [
          "kernel.log",
          file(`Jul 27 00:00:01 jst kernel: process=31337 state=missing
Jul 27 00:00:02 jst kernel: heartbeat=0xC0FFEE payload=messages/.core.b64
Jul 27 00:00:03 jst kernel: core_dump=0
`),
        ],
      ]),
    ],
    [
      "messages",
      directory([
        [
          ".core.b64",
          file(`SlNUX1FVRVNUX0NPTVBMRVRFX1YxCg==
`),
        ],
        [
          "URGENT_DO_NOT_DECODE.b64",
          file(`SlNUX1JJQ0tST0xMX1YxCg==
`),
        ],
      ]),
    ],
    [
      "museum",
      directory([
        [
          "left.txt",
          file(`exhibit=404
bias=left
`),
        ],
        [
          "right.txt",
          file(`exhibit=404
bias=right
`),
        ],
      ]),
    ],
    [
      "projects",
      directory([
        [
          "jst",
          directory([
            [
              "src",
              directory([
                [
                  "main.rs",
                  file(`fn main() {
    println!("jst do this.");
}
`),
                ],
              ]),
            ],
          ]),
        ],
      ]),
    ],
  ]);
  const preopen = new PreopenDirectory(".", root);
  const now = BigInt(Date.now()) * 1_000_000n;
  const stamp = (inode) => {
    const originalStat = inode.stat.bind(inode);
    inode.stat = () => {
      const stat = originalStat();
      stat.atim = now;
      stat.mtim = now;
      stat.ctim = now;
      return stat;
    };
    if (inode.contents instanceof Map) {
      for (const child of inode.contents.values()) stamp(child);
    }
  };
  stamp(preopen.dir);
  return preopen;
}

async function compileModule(url) {
  const response = await fetch(url, { cache: "force-cache" });
  if (!response.ok) throw new Error(`A Linux tool failed to load (${response.status}).`);
  if (WebAssembly.compileStreaming) {
    try {
      return await WebAssembly.compileStreaming(response.clone());
    } catch {
      // Static development servers may omit application/wasm.
    }
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

async function boot() {
  progress("runtime");
  workspace = buildWorkspace();
  workingDirectory = [];
  progress("package");
  const compiled = await Promise.all(
    Object.entries(moduleUrls).map(async ([name, url]) => [
      name,
      await compileModule(url),
    ]),
  );
  modules = Object.fromEntries(compiled);
  progress("shell");
  progress("ready");
}

function concatBytes(chunks) {
  const length = chunks.reduce((total, chunk) => total + chunk.length, 0);
  const output = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.length;
  }
  return decoder.decode(output);
}

async function runWasi(
  module,
  argv,
  stdin,
  outputBudget,
  runWorkspace,
  stdoutIsTerminal = true,
) {
  const stdoutChunks = [];
  const stderrChunks = [];
  const stdoutFile = new File(new Uint8Array());
  const capture = (chunks) => (bytes) => {
    outputBudget.consume(bytes.byteLength);
    chunks.push(new Uint8Array(bytes));
  };
  const fds = [
    new OpenFile(new File(encoder.encode(stdin), { readonly: true })),
    // A real shell exposes non-final pipeline stdout as a non-TTY. Utilities
    // such as `ls` use that distinction to switch from columns to one item per line.
    stdoutIsTerminal
      ? new ConsoleStdout(capture(stdoutChunks))
      : new BoundedOpenFile(stdoutFile, outputBudget),
    new ConsoleStdout(capture(stderrChunks)),
    runWorkspace,
  ];
  const environment = ["LANG=C.UTF-8", "LC_ALL=C.UTF-8", "NO_COLOR=1", "TERM=dumb"];
  const wasi = new WASI(argv, environment, fds);
  installWasiRandomGet(wasi);

  // browser_wasi_shim 0.4.0 reports UTF-16 string lengths where WASI expects
  // UTF-8 byte lengths. Patch the two size calls before instantiation.
  wasi.wasiImport.args_sizes_get = function argsSizesGet(countPointer, sizePointer) {
    const memory = () => new DataView(wasi.inst.exports.memory.buffer);
    memory().setUint32(countPointer, argv.length, true);
    const size = argv.reduce((total, argument) => total + encoder.encode(argument).length + 1, 0);
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

  // Clear the filestat padding bytes that Rust reads from MaybeUninit.
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
    wasi.start(instance.instance || instance);
  } catch (error) {
    if (error instanceof WASIProcExit) code = error.code;
    else throw error;
  }
  return {
    code,
    stderr: concatBytes(stderrChunks),
    stdout: stdoutIsTerminal ? concatBytes(stdoutChunks) : decoder.decode(stdoutFile.data),
  };
}

async function run(command) {
  if (!modules || !workspace) throw new Error("The Linux tools did not finish loading.");

  const parsedCommand = parseDemoCommand(command);
  if (
    parsedCommand.type === "pipeline" &&
    parsedCommand.pipeline[0]?.name === "cd"
  ) {
    workingDirectory = resolveWorkspaceDirectory(
      workspace,
      workingDirectory,
      parsedCommand.pipeline[0].args[0] || "/",
    );
    return {
      code: 0,
      cwd: workingDirectory.join("/"),
      outputPath: null,
      redirectedStdout: "",
      stderr: "",
      stdout: "",
    };
  }
  const activeWorkspace = workspaceAtDirectory(workspace, workingDirectory);
  const loopMatches = parsedCommand.type === "for-each-cat"
    ? expandWorkspaceGlob(activeWorkspace, parsedCommand.glob)
    : null;
  const pipeline = parsedCommand.type === "for-each-cat"
    ? [{
        args: loopMatches.length ? loopMatches : [parsedCommand.glob],
        inputPath: null,
        name: "cat",
        outputPath: null,
      }]
    : parsedCommand.pipeline;
  let stdin = "";
  let stderr = "";
  let code = 0;
  let outputPath = null;
  let redirectedStdout = "";
  const outputBudget = new OutputBudget(MAX_OUTPUT_BYTES);
  for (const [index, segment] of pipeline.entries()) {
    const { args, globIndexes, inputPath, name } = segment;
    if (inputPath) stdin = readWorkspaceFile(activeWorkspace, inputPath);
    const moduleName = DEMO_STANDALONE_COMMANDS.get(name) || "coreutils";
    const expandedArgs = expandWorkspaceArguments(
      activeWorkspace,
      args,
      globIndexes,
    );
    const runtimeArgs = demoRuntimeArguments(name, expandedArgs);
    const argv = moduleName === "coreutils"
      ? ["coreutils", name, ...runtimeArgs]
      : [name, ...runtimeArgs];
    const output = await runWasi(
      modules[moduleName],
      argv,
      stdin,
      outputBudget,
      activeWorkspace,
      index === pipeline.length - 1 && !segment.outputPath,
    );
    code = output.code;
    stdin = output.stdout;
    stderr += output.stderr;
    if (encoder.encode(stdin + stderr).byteLength > MAX_OUTPUT_BYTES) {
      throw new Error("The command produced too much output, so the sandbox was reset.");
    }
    if (segment.outputPath) {
      writeWorkspaceFile(activeWorkspace, segment.outputPath, stdin);
      outputPath = segment.outputPath;
      redirectedStdout = stdin;
      stdin = "";
    }
    if (code !== 0) break;
  }
  return {
    code,
    cwd: workingDirectory.join("/"),
    outputPath,
    redirectedStdout,
    stderr,
    stdout: stdin,
  };
}

async function handleWorkerMessage(message) {
  if (message?.type === "boot") {
    try {
      await boot();
      post({ type: "ready" });
    } catch (error) {
      post({
        type: "boot-error",
        message: error instanceof Error ? error.message : "The sandbox failed to start.",
      });
    }
  } else if (message?.type === "run") {
    try {
      const output = await run(message.command);
      post({
        type: "output",
        code: output.code,
        cwd: output.cwd,
        outputPath: output.outputPath,
        redirectedStdout: output.redirectedStdout,
        stderr: output.stderr,
        stdout: output.stdout,
      });
    } catch (error) {
      post({
        type: "run-error",
        message: error instanceof Error ? error.message : "The command failed.",
      });
    }
  }
}

if (isWorker) {
  self.addEventListener("message", (event) => {
    void handleWorkerMessage(event.data);
  });
} else {
  let worker = null;
  let workerReady = false;

  window.addEventListener("message", (event) => {
    if (
      event.source !== parent ||
      event.origin !== location.origin ||
      event.data?.channel !== channel
    ) {
      return;
    }

    if (event.data?.type === "boot") {
      worker = new Worker("/assets/demo-sandbox.js?v=15", { type: "module" });
      worker.addEventListener("message", (workerEvent) => {
        if (workerEvent.data?.type === "ready") workerReady = true;
        post(workerEvent.data);
      });
      worker.addEventListener("error", () => {
        post({
          type: workerReady ? "run-error" : "boot-error",
          message: workerReady
            ? "The command failed."
            : "The sandbox failed to start.",
        });
      });
      worker.postMessage({ type: "boot" });
    } else if (event.data?.type === "run") {
      if (worker) {
        worker.postMessage({ command: event.data.command, type: "run" });
      } else {
        post({ type: "run-error", message: "The sandbox is not available." });
      }
    } else if (event.data?.type === "destroy") {
      worker?.terminate();
      worker = null;
    }
  });

  post({ type: "loaded" });
}
