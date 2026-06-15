# loom

loom はマークダウンライクなテキストベース作曲用の DSL である

必要に応じて、docs/llms.md を参照して loom を理解すること

## DevOps

- mise でツールを管理する
- just でタスクを管理する
- 実装が終了後、 `just ci` コマンドで品質チェックをする
    - ドキュメントのみの変更の場合は不要
- コミットメッセージは `type: 日本語の概要` の形式にする
    - type は Conventional Commits に従う

## Studio

- loom 用の TUI ツール
- 実装に合わせて以下を行う
    - Footer のヘルプを合わせる
    - docs/guide/studio.md の記述を合わせる
- 入力は KeyStroke で抽象化する
    - 端末差を吸収し、ロジックに集中するため
    - KeyStroke と KeyAction を紐づける

## 公開ドキュメント

- docs/ で loom の公開ドキュメントを管理する
- docs/ は VitePress によってデプロイされる
- docs/examples/live-coding/ は loom の言語仕様を網羅するように feature-*.loom ファイルを作成する
    - これは自動的にドキュメントに組み込まれる
    - テストはゴールデンテストを採用している

## 備考

- memo*.md は実装のための個人的なメモで、必要に応じて閲覧、編集して良い
- DSL の実装が変更されたときや、不備が見つかった場合は、 docs/llms.md を変更すること

