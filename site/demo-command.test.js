import assert from "node:assert/strict";
import test from "node:test";
import {
  demoRuntimeArguments,
  isAllowedDemoCommand,
  parseDemoCommand,
  parseDemoPipeline,
} from "./demo-command.js";

test("accepts a bounded loop that cats files from one relative glob", () => {
  const command = 'for file in museum/*.txt; do cat "$file"; done';

  assert.equal(isAllowedDemoCommand(command), true);
  assert.deepEqual(parseDemoCommand(command), {
    glob: "museum/*.txt",
    type: "for-each-cat",
  });
});

test("accepts JST's bounded cat loop with a conventional ./ prefix", () => {
  const command = 'for file in ./museum/*.txt; do cat "$file"; done';

  assert.equal(isAllowedDemoCommand(command), true);
  assert.deepEqual(parseDemoCommand(command), {
    glob: "./museum/*.txt",
    type: "for-each-cat",
  });
});

test("rejects loops that can execute arbitrary shell syntax", () => {
  const rejected = [
    'for file in museum/*.txt; do rm "$file"; done',
    'for file in museum/*.txt; do cat "$other"; done',
    'for file in ../museum/*.txt; do cat "$file"; done',
    'for file in /museum/*.txt; do cat "$file"; done',
    'for file in museum/*.txt; do cat "$file"; echo nope; done',
    'for file in museum/*.txt; do cat "$file"; done; pwd',
  ];

  for (const command of rejected) {
    assert.equal(isAllowedDemoCommand(command), false, command);
  }
});

test("tracks unquoted pathname globs without expanding quoted command patterns", () => {
  const [listing] = parseDemoPipeline("ls -d .??*");
  const [finding] = parseDemoPipeline("find . -name '*.rs'");

  assert.deepEqual(listing.globIndexes, [1]);
  assert.equal(finding.globIndexes, undefined);
});

test("accepts JST's simple decode-to-file translation", () => {
  const command =
    "base64 --decode messages/URGENT_DO_NOT_DECODE.b64 > messages/URGENT_DO_NOT_DECODE.txt";

  assert.equal(isAllowedDemoCommand(command), true);
  assert.deepEqual(parseDemoPipeline(command), [
    {
      args: ["--decode", "messages/URGENT_DO_NOT_DECODE.b64"],
      inputPath: null,
      name: "base64",
      outputPath: "messages/URGENT_DO_NOT_DECODE.txt",
    },
  ]);
});

test("accepts sed substitutions and a standalone cd", () => {
  assert.deepEqual(
    parseDemoPipeline("sed -i 's/0xC0FFEE/0xBADC0DE/g' messages/.core.b64"),
    [
      {
        args: [
          "-i",
          "s/0xC0FFEE/0xBADC0DE/g",
          "messages/.core.b64",
        ],
        inputPath: null,
        name: "sed",
        outputPath: null,
      },
    ],
  );
  assert.deepEqual(parseDemoPipeline("cd messages"), [
    {
      args: ["messages"],
      inputPath: null,
      name: "cd",
      outputPath: null,
    },
  ]);
  assert.deepEqual(
    demoRuntimeArguments("sed", [
      "-i",
      "s/0xC0FFEE/0xBADC0DE/g",
      "messages/.core.b64",
    ]),
    [
      "-i",
      "-e",
      "s/0xC0FFEE/0xBADC0DE/g",
      "messages/.core.b64",
    ],
  );
});

test("keeps output redirection inside one relative sandbox path", () => {
  const rejected = [
    "base64 -d message.b64 > /tmp/message.txt",
    "base64 -d message.b64 > ../message.txt",
    "base64 -d message.b64 >> message.txt",
    "base64 -d message.b64 > message.txt | cat",
    "base64 -d message.b64 > message.txt > second.txt",
  ];

  for (const command of rejected) {
    assert.equal(isAllowedDemoCommand(command), false, command);
  }
});
