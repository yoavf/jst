export function createReminderCopyState({ clearTimeout, setTimeout, setState }) {
  let operation = 0;
  let resetTimer = null;

  function clearResetTimer() {
    if (resetTimer === null) return;
    clearTimeout(resetTimer);
    resetTimer = null;
  }

  function isCurrent(copyOperation) {
    return copyOperation === operation;
  }

  return {
    begin() {
      operation += 1;
      clearResetTimer();
      return operation;
    },
    fail(copyOperation) {
      if (!isCurrent(copyOperation)) return false;
      setState("couldn’t copy");
      return true;
    },
    reset() {
      operation += 1;
      clearResetTimer();
      setState("copy");
    },
    succeed(copyOperation) {
      if (!isCurrent(copyOperation)) return false;
      setState("copied");
      resetTimer = setTimeout(() => {
        if (!isCurrent(copyOperation)) return;
        setState("copy");
        resetTimer = null;
      }, 1800);
      return true;
    },
  };
}
