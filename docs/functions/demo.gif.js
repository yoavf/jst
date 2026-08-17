const DEMO_FILES = [
  "changed-today.gif",
  "clear-port-8080.gif",
  "largest-files.gif",
  "remove-ds-store.gif",
  "zip-folder.gif",
];

export function demoFileAt(index) {
  return DEMO_FILES[index % DEMO_FILES.length];
}

export function demoRedirect(index) {
  return new Response(null, {
    status: 302,
    headers: {
      "Cache-Control": "no-store",
      Location: `/demos/${demoFileAt(index)}`,
    },
  });
}

export function onRequestGet() {
  return demoRedirect(Math.floor(Math.random() * DEMO_FILES.length));
}
