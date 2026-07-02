# Playground

Playground は loom のブラウザ用 editor shell である。編集、compile、format、再生、共有、import/export を扱うため、状態遷移と副作用の境界を明確に保つ。

## 設計方針

- Preact は view layer に限定し、application state の置き場にしない。
- 状態管理は Redux / XState / router framework などを入れず、小さな Elmish runtime で扱う。
    - 基本形は `Model -> Message -> [Model, Command[]]`。
- CSS / visual base は Oat と `src/styles.css` を使う。
- 状態変更は `domain/model.ts` の `update(model, message)` を通す。
- component 内で `dirty`, `compileStatus`, `compiledEvents`, `diagnostics`, `files` などを直接更新しない。
- wasm / audio / clipboard / ZIP / prompt / timer などの副作用は `effects/effects.ts` に閉じ込める。
- `app/runtime.ts` は小さな Elmish runtime として扱う。framework 化しない。

## フォルダ構造

```text
src
├── main.tsx
├── app
│   ├── App.tsx
│   ├── Editor.tsx
│   └── runtime.ts
├── domain
│   ├── model.ts
│   ├── types.ts
│   ├── selectors.ts
│   ├── commands.ts
│   ├── playback.ts
│   ├── reducers
│   │   ├── compiler.ts
│   │   ├── editor.ts
│   │   ├── file.ts
│   │   ├── workspace.ts
│   │   ├── playback.ts
│   │   └── state.ts
│   └── model.test.ts
├── effects
│   └── effects.ts
├── data
│   └── examples.ts
├── audio
├── compiler
├── share
├── workspace
└── generated
```

- `main.tsx` は Preact mount entrypoint。app の起動だけを書く。
- `app/` は Preact components と UI adapter を置く。
- `domain/` は UI から独立した状態、状態遷移、Command 定義を置く。`model.ts` は public entrypoint として `update`, `initModel`, re-export, Message routing を持つ。
- `effects/` は browser API / wasm / audio / ZIP などの副作用実装を置く。
- `audio/`, `compiler/`, `share/`, `workspace/` は独立 helper として扱い、必要以上に `app/` や `domain/` へ寄せない。
- `generated/` は生成物なので手で編集しない。

## Message / Command の扱い

- `Message` は user intent と effect result を表す。
- `Message` を増やす時は、まず既存の責務別 union (`compiler`, `editor`, `file`, `workspace`, `playback`) に追加できるか確認する。
- `CompilerMessage`, `EditorMessage`, `FileMessage`, `WorkspaceMessage`, `PlaybackMessage` などの責務別 Message 型に合わせて `domain/reducers/*.ts` を分ける。
- `domain/reducers/state.ts` だけは例外として、reducer 間で共有する純粋な状態 helper を置く。
- `domain/model.ts` の `update()` は Message routing を一覧できる場所として維持し、case の実装詳細を増やさない。
- `domain/reducers/*.ts` から `domain/model.ts` を import しない。必要な型や helper は `types.ts`, `selectors.ts`, `commands.ts`, `playback.ts`, `reducers/state.ts` から import する。
- component は `dispatch(message)` だけ行う。
- prompt / confirm のような分岐は component に書かず、`Command` + `Effects` に寄せる。
- `Command` は副作用を実行して、必要なら result message を dispatch する。
- model test では `Command` の中身を実行しない。返された command の種類や、model の変化を確認する。

## CodeMirror

- CodeMirror は `app/Editor.tsx` に閉じ込める。
- source edit は `source-changed` message として model に送る。
- diagnostics decoration は model の diagnostics から作る。
- active file / external content update / pending cursor の同期は Editor adapter で扱う。
- workspace state 更新を CodeMirror callback 内に書かない。

## 変更時の確認

- Playground を変更したら最低限 `just playground::test` と `just playground::app-build` を通す。
- ブラウザ上の挙動確認が必要な場合は Playwright MCP を使い、`just playground::dev` で起動して確認する。
- 手動 QA では初期 compile、編集、format、diagnostics、share、export、play/stop、mobile layout を必要に応じて確認する。
- スクリーンショットは一時的な確認証跡として扱う。残す場合は ignore 済みの `.playwright-mcp/screenshots/` に保存し、ユーザー向けの報告でパスを明示する。
