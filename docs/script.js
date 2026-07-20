const examples = [
  {
    request: "jst find all files bigger than 500 mb in ~/downloads",
    result: "find ~/downloads -type f -size +500M",
  },
  {
    request: "jst show me what's using port 3000",
    result: "lsof -i :3000",
  },
  {
    request: "jst show the 10 largest files in this folder",
    result: "du -ah . | sort -hr | head -n 10",
  },
  {
    request: "jst find every TODO in rust files",
    result: "rg --glob '*.rs' TODO",
  },
  {
    request: "jst list git branches by most recent activity",
    result: "git branch --sort=-committerdate",
  },
  {
    request: "jst find files changed in the last 24 hours",
    result: "find . -type f -mtime -1",
  },
  {
    request: "jst make a receipts folder and move all PDFs into it",
    result: "mkdir -p receipts && mv -- *.pdf receipts/",
  },
  {
    request: "jst show the last 20 lines of my application log",
    result: "tail -n 20 application.log",
  },
  {
    request: "jst count the lines in every rust file",
    result: "find . -name '*.rs' -print0 | xargs -0 wc -l",
  },
];

const requestElement = document.querySelector("#request");
const resultElement = document.querySelector("#result");
const translationElement = document.querySelector(".translation");
const resultLine = document.querySelector(".terminal-line--result");
const anotherButton = document.querySelector(".another-example");
const copyButton = document.querySelector(".install-command");
const copyState = document.querySelector(".copy-state");
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

let exampleIndex = 0;
let animationRun = 0;

function wait(milliseconds) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

async function typeText(element, value, delay, run) {
  element.textContent = "";

  for (const character of value) {
    if (run !== animationRun) return false;
    element.textContent += character;
    await wait(delay);
  }

  return true;
}

async function showExample(index, animate = true) {
  const example = examples[index];
  const run = ++animationRun;
  translationElement.setAttribute(
    "aria-label",
    `JST turns ${example.request} into the shell command ${example.result}`,
  );

  if (!animate || reduceMotion.matches) {
    requestElement.textContent = example.request;
    resultElement.textContent = example.result;
    resultLine.classList.add("is-visible");
    return;
  }

  requestElement.textContent = "";
  resultElement.textContent = "";
  resultLine.classList.remove("is-visible");

  await wait(220);
  const requestFinished = await typeText(requestElement, example.request, 24, run);
  if (!requestFinished || run !== animationRun) return;

  await wait(360);
  resultLine.classList.add("is-visible");
  await typeText(resultElement, example.result, 17, run);
}

anotherButton?.addEventListener("click", () => {
  exampleIndex = (exampleIndex + 1) % examples.length;
  showExample(exampleIndex);
});

copyButton?.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(copyButton.dataset.copy);
    copyState.textContent = "Copied";
    window.setTimeout(() => {
      copyState.textContent = "copy";
    }, 1800);
  } catch {
    copyState.textContent = "select + copy";
  }
});

window.requestAnimationFrame(() => showExample(0));
