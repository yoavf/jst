export const DEMO_COREUTILS_COMMANDS = new Set([
  "arch", "b2sum", "base32", "base64", "basename", "basenc", "cat", "cksum",
  "comm", "cp", "cut", "date", "dir", "dircolors", "dirname", "echo", "expand",
  "factor", "false", "fmt", "fold", "head", "join", "link", "ln", "ls",
  "md5sum", "mkdir", "mktemp", "mv", "nl", "nproc", "numfmt", "od", "paste",
  "pathchk", "pr", "printenv", "printf", "ptx", "pwd", "readlink", "realpath",
  "rm", "rmdir", "seq", "sha1sum", "sha224sum", "sha256sum", "sha384sum",
  "sha512sum", "shuf", "sort", "sum", "tail", "touch", "tr", "true", "tsort",
  "tty", "uname", "unexpand", "uniq", "unlink", "vdir", "wc",
]);

export const DEMO_STANDALONE_COMMANDS = new Map([
  ["cmp", "diffutils"],
  ["column", "column"],
  ["diff", "diffutils"],
  ["find", "find"],
  ["grep", "grep"],
]);

const UNSAFE_DEMO_COMMAND = /[\\`;&$(){}!#\u0000-\u001f\u007f]/;
const FOR_EACH_CAT_COMMAND =
  /^for\s+([A-Za-z_][A-Za-z0-9_]*)\s+in\s+([A-Za-z0-9._/-]*\*[A-Za-z0-9._*-]*)\s*;\s*do\s+cat\s+"\$\1"\s*;\s*done\s*$/;

function parseForEachCat(command) {
  const match = command.match(FOR_EACH_CAT_COMMAND);
  if (!match) return null;

  const glob = match[2];
  const components = glob.split("/");
  const starCount = [...glob].filter((character) => character === "*").length;
  if (
    glob.startsWith("/") ||
    starCount !== 1 ||
    components.some((component) => !component || component === "." || component === "..") ||
    components.slice(0, -1).some((component) => component.includes("*"))
  ) {
    return null;
  }
  return { glob, type: "for-each-cat" };
}

function parseWords(segment) {
  const words = [];
  let word = "";
  let quote = null;
  let started = false;
  let hasUnquotedGlob = false;
  for (const character of segment.trim()) {
    if (quote) {
      if (character === quote) quote = null;
      else word += character;
    } else if (character === "'" || character === '"') {
      quote = character;
      started = true;
    } else if (/\s/.test(character)) {
      if (started) {
        words.push({ hasUnquotedGlob, value: word });
        word = "";
        started = false;
        hasUnquotedGlob = false;
      }
    } else {
      if (character === "*" || character === "?") hasUnquotedGlob = true;
      word += character;
      started = true;
    }
  }
  if (quote) throw new Error("The translated command contains an unmatched quote.");
  if (started) words.push({ hasUnquotedGlob, value: word });
  return words;
}

function hasUnsafeArguments(name, args) {
  if (name === "find") {
    return args.some((argument) =>
      [
        "-delete",
        "-exec",
        "-execdir",
        "-fls",
        "-fprint",
        "-fprint0",
        "-fprintf",
        "-ok",
        "-okdir",
      ].includes(argument),
    );
  }
  if (name === "sort") {
    return args.some(
      (argument) =>
        argument === "-o" ||
        argument.startsWith("-o") ||
        argument === "--output" ||
        argument.startsWith("--output=") ||
        argument === "--compress-program" ||
        argument.startsWith("--compress-program="),
    );
  }
  return false;
}

export function parseDemoPipeline(command) {
  if (
    typeof command !== "string" ||
    !command.trim() ||
    command.includes("||") ||
    UNSAFE_DEMO_COMMAND.test(command)
  ) {
    throw new Error("That command is outside the browser toolbox.");
  }

  const segments = command.split("|");
  return segments.map((segment, segmentIndex) => {
    const words = parseWords(segment);
    if (
      words.some(
        ({ value }) =>
          (value.includes("<") && value !== "<") ||
          (value.includes(">") && value !== ">"),
      )
    ) {
      throw new Error("Only simple file redirection is available.");
    }
    const commandWords = [];
    let inputPath = null;
    let outputPath = null;
    for (let index = 0; index < words.length; index += 1) {
      const word = words[index].value;
      if (word !== "<" && word !== ">") {
        commandWords.push(words[index]);
        continue;
      }
      const path = words[index + 1]?.value;
      const isOutput = word === ">";
      if (
        commandWords.length === 0 ||
        !path ||
        path === "<" ||
        path === ">" ||
        path.startsWith("/") ||
        path.split("/").some((component) => !component || component === "..") ||
        (isOutput ? outputPath !== null : inputPath !== null) ||
        (isOutput && segmentIndex !== segments.length - 1)
      ) {
        throw new Error("Only simple file redirection is available.");
      }
      if (isOutput) outputPath = path;
      else inputPath = path;
      index += 1;
    }
    const [nameWord, ...argWords] = commandWords;
    const name = nameWord?.value;
    const args = argWords.map(({ value }) => value);
    const globIndexes = argWords.flatMap((word, index) =>
      word.hasUnquotedGlob ? [index] : [],
    );
    const available =
      DEMO_COREUTILS_COMMANDS.has(name) || DEMO_STANDALONE_COMMANDS.has(name);
    if (!name || !available || hasUnsafeArguments(name, args)) {
      throw new Error("That command is outside the browser toolbox.");
    }
    return {
      args,
      ...(globIndexes.length ? { globIndexes } : {}),
      inputPath,
      name,
      outputPath,
    };
  });
}

export function parseDemoCommand(command) {
  if (typeof command !== "string" || !command.trim()) {
    throw new Error("That command is outside the browser toolbox.");
  }
  const forEachCat = parseForEachCat(command.trim());
  if (forEachCat) return forEachCat;
  return { pipeline: parseDemoPipeline(command), type: "pipeline" };
}

export function isAllowedDemoCommand(command) {
  try {
    parseDemoCommand(command);
    return true;
  } catch {
    return false;
  }
}
