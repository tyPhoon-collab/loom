import "@knadh/oat/oat.min.css";
import "./styles.css";

type PlaygroundFile = {
  path: string;
  content: string;
};

type Diagnostic = {
  path?: string;
  line?: number;
  message: string;
};

const files: PlaygroundFile[] = [
  {
    path: "song.loom",
    content: `---
title: Minimal Melody
---

# Lead: 1
C4 | ^ . ^ . |
`,
  },
  {
    path: "sections/intro.loom",
    content: "# 1\nC4 | ^ . ^ . |\n",
  },
];

const diagnostics: Diagnostic[] = [
  {
    path: "song.loom",
    line: 6,
    message: "Playground scaffold is ready. Core compile is not wired yet.",
  },
];

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing #app root");
}

app.innerHTML = `
  <main class="playground-shell">
    <aside class="workspace-panel" aria-label="Workspace files">
      <header class="panel-header">
        <div>
          <p class="eyebrow">Workspace</p>
          <h1>Loom Playground</h1>
        </div>
        <button type="button" class="small">New</button>
      </header>
      <nav class="file-list" aria-label="Files">
        ${files
          .map(
            (file, index) => `
              <button type="button" class="file-item ${index === 0 ? "active" : ""}">
                <span>${file.path}</span>
              </button>
            `,
          )
          .join("")}
      </nav>
    </aside>

    <section class="editor-panel" aria-label="Loom source editor">
      <div class="toolbar" role="toolbar" aria-label="Playground actions">
        <button type="button">Play</button>
        <button type="button">Stop</button>
        <button type="button">Format</button>
        <button type="button">Share</button>
        <button type="button">Export ZIP</button>
      </div>
      <textarea spellcheck="false" aria-label="song.loom source">${files[0].content}</textarea>
    </section>

    <aside class="diagnostics-panel" aria-label="Diagnostics">
      <header class="panel-header compact">
        <div>
          <p class="eyebrow">Compile</p>
          <h2>Diagnostics</h2>
        </div>
        <span class="status-pill">Scaffold</span>
      </header>
      <ol class="diagnostics-list">
        ${diagnostics
          .map(
            (diagnostic) => `
              <li>
                <strong>${diagnostic.path ?? "workspace"}${diagnostic.line ? `:${diagnostic.line}` : ""}</strong>
                <span>${diagnostic.message}</span>
              </li>
            `,
          )
          .join("")}
      </ol>
    </aside>
  </main>
`;
