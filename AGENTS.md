# loom

loom はマークダウンライクなテキストベース作曲用の DSL である

## DevOps

- mise でツールを管理する
- just でタスクを管理する
- 一通り実装が終了したときは `just ci` コマンドで品質チェックをする

## Studio

- loom 用の TUI ツール
- 実装に合わせて以下を行う
    - Footer のヘルプを合わせる
    - docs/guide/studio.md の記述を合わせる

## 公開ドキュメント

- docs/ で loom の公開ドキュメントを管理する
- docs/ は VitePress によってデプロイされる
- docs/examples/live-coding/ は loom の言語仕様を網羅するように feature-*.loom ファイルを作成する
    - これは自動的にドキュメントに組み込まれる
    - テストはゴールデンテストを採用している

## 備考

- memo*.md は実装のための個人的なメモで、必要に応じて閲覧、編集して良い
