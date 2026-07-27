import {
  Directory,
  File,
  PreopenDirectory,
} from "@bjorn3/browser_wasi_shim";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function pathComponents(path) {
  const components = path.split("/").filter((component) => component !== ".");
  if (
    !path ||
    path.startsWith("/") ||
    components.length === 0 ||
    components.some((component) => !component || component === "..")
  ) {
    throw new Error(`${path}: not a safe sandbox path`);
  }
  return components;
}

function directoryAt(workspace, components, originalPath) {
  let entry = workspace.dir;
  for (const component of components) {
    if (!(entry instanceof Directory)) {
      throw new Error(`${originalPath}: not a directory`);
    }
    entry = entry.contents.get(component);
    if (!entry) throw new Error(`${originalPath}: no such sandbox directory`);
  }
  if (!(entry instanceof Directory)) {
    throw new Error(`${originalPath}: not a directory`);
  }
  return entry;
}

export function resolveWorkspaceDirectory(workspace, currentComponents, path) {
  if (
    typeof path !== "string" ||
    !path ||
    path.includes("\u0000")
  ) {
    throw new Error(`${path}: not a safe sandbox directory`);
  }

  let requested = path;
  let components = [...currentComponents];
  if (requested === "~" || requested === "~/playground") {
    requested = "/";
  } else if (requested.startsWith("~/playground/")) {
    requested = requested.slice("~/playground".length);
  }
  if (requested.startsWith("/")) components = [];

  for (const component of requested.split("/")) {
    if (!component || component === ".") continue;
    if (component === "..") {
      components.pop();
      continue;
    }
    components.push(component);
  }

  directoryAt(workspace, components, path);
  return components;
}

export function workspaceAtDirectory(workspace, components) {
  const directory = directoryAt(workspace, components, components.join("/") || "/");
  return new PreopenDirectory(".", directory.contents);
}

export function readWorkspaceFile(workspace, path) {
  let entry = workspace.dir;
  for (const component of pathComponents(path)) {
    if (!(entry instanceof Directory)) {
      throw new Error(`${path}: not a readable sandbox file`);
    }
    entry = entry.contents.get(component);
    if (!entry) throw new Error(`${path}: no such sandbox file`);
  }
  if (!(entry instanceof File)) throw new Error(`${path}: not a regular file`);
  return decoder.decode(entry.data);
}

export function writeWorkspaceFile(workspace, path, contents) {
  const components = pathComponents(path);
  const filename = components.pop();
  let parent = workspace.dir;
  for (const component of components) {
    if (!(parent instanceof Directory)) {
      throw new Error(`${path}: parent is not a directory`);
    }
    parent = parent.contents.get(component);
    if (!parent) throw new Error(`${path}: parent directory does not exist`);
  }
  if (!(parent instanceof Directory)) {
    throw new Error(`${path}: parent is not a directory`);
  }
  const existing = parent.contents.get(filename);
  if (existing instanceof Directory) {
    throw new Error(`${path}: is a directory`);
  }
  parent.contents.set(filename, new File(encoder.encode(contents)));
}

export function expandWorkspaceGlob(workspace, pattern) {
  const components = pathComponents(pattern);
  const filenamePattern = components.pop();
  if (
    !/[*?]/.test(filenamePattern) ||
    components.some((component) => /[*?]/.test(component))
  ) {
    throw new Error(`${pattern}: not a supported sandbox glob`);
  }

  let parent = workspace.dir;
  for (const component of components) {
    if (!(parent instanceof Directory)) return [];
    parent = parent.contents.get(component);
    if (!parent) return [];
  }
  if (!(parent instanceof Directory)) return [];

  let expressionSource = "";
  for (const character of filenamePattern) {
    if (character === "*") expressionSource += ".*";
    else if (character === "?") expressionSource += ".";
    else expressionSource += character.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }
  const expression = new RegExp(`^${expressionSource}$`);
  const prefix = components.length ? `${components.join("/")}/` : "";
  const matches = [...parent.contents.entries()]
    .filter(
      ([name, entry]) =>
        (entry instanceof File || entry instanceof Directory) &&
        (filenamePattern.startsWith(".") || !name.startsWith(".")) &&
        expression.test(name),
    )
    .map(([name]) => `${prefix}${name}`)
    .sort();
  if (matches.length > 128) {
    throw new Error(`${pattern}: matched too many sandbox paths`);
  }
  return matches;
}

export function expandWorkspaceArguments(workspace, args, globIndexes = []) {
  const indexes = new Set(globIndexes);
  return args.flatMap((argument, index) => {
    if (!indexes.has(index)) return [argument];
    const matches = expandWorkspaceGlob(workspace, argument);
    return matches.length ? matches : [argument];
  });
}
