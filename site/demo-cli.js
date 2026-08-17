export const JST_VERSION = "0.4.0";

export const JST_HELP = `Turn plain English into a shell command and run it

Usage: jst [OPTIONS] [PROMPT]...

Arguments:
  [PROMPT]...  What you want to do, in plain English

Options:
      --yolo         Skip all safety confirmations
  -i, --interactive  Review the command before running
      --dry          Show the generated command without running it
      --status       Check server health, models, and aggregate usage
  -h, --help         Print help
  -V, --version      Print version

Examples:
  jst show the 10 largest files here
  jst --dry find files containing BANANA
  jst -i create a photos folder
  jst --status

This playground runs commands only inside its disposable browser sandbox.`;

function parseWords(value) {
  const words = [];
  let word = "";
  let quote = null;
  let started = false;
  for (const character of value.trim()) {
    if (quote) {
      if (character === quote) quote = null;
      else word += character;
    } else if (character === "'" || character === '"') {
      quote = character;
      started = true;
    } else if (/\s/.test(character)) {
      if (started) {
        words.push(word);
        word = "";
        started = false;
      }
    } else {
      word += character;
      started = true;
    }
  }
  if (quote) return null;
  if (started) words.push(word);
  return words;
}

export function parseJstInvocation(value) {
  const words = parseWords(value);
  if (!words) return { error: "unexpected argument: unmatched quote" };
  if (words.length === 0) return { action: "help" };

  const options = {
    dry: false,
    interactive: false,
    status: false,
    yolo: false,
  };
  let index = 0;
  while (index < words.length) {
    const word = words[index];
    if (word === "--") {
      index += 1;
      break;
    }
    if (!word.startsWith("-") || word === "-") break;
    if (word === "-h" || word === "--help") return { action: "help" };
    if (word === "-V" || word === "--version") return { action: "version" };
    if (word === "--yolo") options.yolo = true;
    else if (word === "-i" || word === "--interactive") options.interactive = true;
    else if (word === "--dry") options.dry = true;
    else if (word === "--status") options.status = true;
    else return { error: `unexpected argument '${word}'` };
    index += 1;
  }

  if (options.yolo && (options.interactive || options.dry)) {
    return { error: "'--yolo' cannot be used with '--interactive' or '--dry'" };
  }
  if (options.interactive && options.dry) {
    return { error: "'--interactive' cannot be used with '--dry'" };
  }

  const promptWords = words.slice(index);
  if (options.status) {
    if (options.yolo || options.interactive || options.dry || promptWords.length > 0) {
      return { error: "'--status' cannot be combined with a prompt or execution options" };
    }
    return { action: "status" };
  }
  if (promptWords.length === 0) {
    return { error: "a prompt is required with execution options" };
  }
  return {
    action: "translate",
    input: promptWords.join(" "),
    ...options,
  };
}
