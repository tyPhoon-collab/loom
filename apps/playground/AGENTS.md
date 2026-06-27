# Playground

Playground は loom のブラウザ用 editor shell である。表示は薄いが、workspace state をまたいで editing / compile / format / playback / share / import / export が動くため、状態遷移と副作用の境界を優先して保つ。

## 設計方針

- UI framework は Preact を使う。
    - Preact は component rendering のために使い、application state の置き場にはしない。
- 状態管理は Redux / XState / router framework などを入れず、小さな Elmish runtime で扱う。
    - 基本形は `Model -> Message -> [Model, Command[]]`。
- CSS / visual base は Oat と `src/styles.css` を使う。
    - Oat は見た目の土台であり、状態管理 framework ではない。
- Preact は view layer に限定する。
- 状態変更は `domain/model.ts` の `update(model, message)` を通す。
- component 内で `dirty`, `compileStatus`, `compiledEvents`, `diagnostics`, `files` などを直接更新しない。
- 副作用は `effects/effects.ts` に閉じ込める。
    - wasm compile / format
    - WebAudio playback
    - clipboard
    - ZIP import / export
    - `window.prompt` / `window.confirm`
    - playback timer
- `app/runtime.ts` は小さな Elmish runtime として扱う。framework 化しない。

## フォルダ構造

- `src/main.tsx`
    - Preact mount entrypoint。
    - app の起動だけを書く。
- `src/app/`
    - Preact components と UI adapter。
    - `App.tsx`: shell, toolbar, file list, diagnostics。
    - `Editor.tsx`: CodeMirror lifecycle adapter。
    - `runtime.ts`: `Model -> Message -> [Model, Command[]]` を実行する小さな runtime。
- `src/domain/`
    - UI から独立した状態と状態遷移。
    - `model.ts`: public entrypoint。`update`, `initModel`, re-export, Message routing だけを書く。
    - `types.ts`: `Model`, `Message`, `Command`, `Effects` などの型定義。
    - `selectors.ts`: `Model` から値を取り出す読み取り helper。
    - `commands.ts`: `Effects` を使う副作用 command。
    - `playback.ts`: 再生 option の純粋な計算 helper。
    - `reducers/`: `Message` の責務別 union と対応する状態遷移。
        - `compiler.ts`: `CompilerMessage`
        - `editor.ts`: `EditorMessage`
        - `file.ts`: `FileMessage`
        - `workspace.ts`: `WorkspaceMessage`
        - `playback.ts`: `PlaybackMessage`
        - `state.ts`: reducer 間で共有する `markDirty`, `fail`, `workspaceDiagnostic` など。
    - `model.test.ts`: state transition のテスト。
- `src/effects/`
    - browser API や wasm などの副作用 adapter。
    - `effects.ts`: `Effects` の実装。
- `src/data/`
    - 静的データ。
    - `examples.ts`: Playground 初期 example workspace。
- `src/audio/`, `src/compiler/`, `src/share/`, `src/workspace/`
    - 既存の独立 helper。必要以上に app/domain へ寄せない。

## Message / Command の扱い

- `Message` は user intent と effect result を表す。
- `Message` を増やす時は、まず既存の責務別 union に追加できるか確認する。
    - compiler
    - editor
    - file
    - workspace
    - playback
- reducer 実装は `Message` の責務別 union に対応する `domain/reducers/*.ts` に置く。
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
