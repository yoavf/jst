import { OpenFile } from "@bjorn3/browser_wasi_shim";

export const OUTPUT_LIMIT_MESSAGE =
  "The command produced too much output, so the sandbox was reset.";

export class OutputLimitError extends Error {
  constructor() {
    super(OUTPUT_LIMIT_MESSAGE);
    this.name = "OutputLimitError";
  }
}

export class OutputBudget {
  constructor(maxBytes) {
    this.maxBytes = maxBytes;
    this.usedBytes = 0;
  }

  consume(byteLength) {
    if (this.usedBytes + byteLength > this.maxBytes) {
      throw new OutputLimitError();
    }
    this.usedBytes += byteLength;
  }
}

export class BoundedOpenFile extends OpenFile {
  constructor(file, budget) {
    super(file);
    this.budget = budget;
  }

  fd_write(data) {
    this.budget.consume(data.byteLength);
    return super.fd_write(data);
  }

  fd_pwrite(data, offset) {
    this.budget.consume(data.byteLength);
    return super.fd_pwrite(data, offset);
  }
}
