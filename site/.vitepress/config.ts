import { defineConfig } from "vitepress";
import { siteUrls } from "./site";

export default defineConfig({
  title: "Loom",
  description: "Text-first MIDI composition DSL",
  lastUpdated: true,
  head: [
    ["link", { rel: "icon", href: "/favicon.svg", type: "image/svg+xml" }],
    ["link", { rel: "alternate icon", href: "/icon.svg", type: "image/svg+xml" }]
  ],
  themeConfig: {
    nav: [
      { text: "Guide", link: "/guide/getting-started" },
      { text: "Language", link: "/language/spec" },
      { text: "Reference", link: "/reference/config" },
      { text: "Examples", link: "/examples/" },
      { text: "Playground", link: siteUrls.playground }
    ],
    sidebar: [
      {
        text: "Guide",
        items: [
          { text: "Getting Started", link: "/guide/getting-started" },
          { text: "Playback", link: "/guide/playback" },
          { text: "CLI", link: "/guide/cli" },
          { text: "Studio", link: "/guide/studio" }
        ]
      },
      {
        text: "Language",
        items: [
          { text: "Specification", link: "/language/spec" },
          { text: "Templates", link: "/language/templates" }
        ]
      },
      {
        text: "Concepts",
        items: [
          { text: "Concept", link: "/concepts/concept" },
          { text: "Philosophy", link: "/concepts/philosophy" },
          { text: "Studio Keybinding Policy", link: "/concepts/studio-keybindings" }
        ]
      },
      {
        text: "Reference",
        items: [
          { text: "Global Config", link: "/reference/config" },
          { text: "General MIDI Programs", link: "/reference/general-midi-programs" },
          { text: "Formatter", link: "/reference/formatter" },
          { text: "Errors", link: "/reference/errors" }
        ]
      },
      {
        text: "Examples",
        items: [
          { text: "Overview", link: "/examples/" },
          { text: "Starter", link: "/examples/starter" },
          { text: "Musical", link: "/examples/musical" },
          { text: "Live Coding", link: "/examples/live-coding" }
        ]
      },
      {
        text: "Tools",
        items: [
          { text: "Playground", link: siteUrls.playground }
        ]
      }
    ]
  }
});
